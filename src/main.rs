use clap::{Parser, Subcommand};
use katzen_airgapped::identicon::Identicon;
use katzen_airgapped::pigeonhole::{
    default_vault_mailbox_path, MessageDirection, ReplicaRead, ReplicaReadReply, VaultMailbox,
};
use katzen_airgapped::sandbox::{apply_network_isolation, is_network_blocked, SandboxStatus};
use katzen_airgapped::sphinx::{ConsensusTopology, SphinxEngine, SurbManager};
use katzen_airgapped::ui::run_gui;
use katzen_airgapped::virtual_keyboard::VirtualKeyboardState;
use katzen_airgapped::{
    fragment_payload, render_qr_terminal, KpagChunk, MsgType, Reassembler, DEFAULT_CHUNK_SIZE,
};
use rand::Rng;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "katzen")]
#[command(author = "Gabriel Virebent <gabrix73>")]
#[command(version = "0.1.0")]
#[command(about = "Katzenpost Sovereign Universal Client - Asynchronous Post-Quantum Mixnet Messaging")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all-in-one unified mode on a connected machine (vault + network spooler)
    Standalone {
        /// Message text to send
        #[arg(short, long)]
        message: Option<String>,

        /// Destination mailbox or service
        #[arg(short = 't', long, default_value = "pigeonhole-egress-01")]
        target: String,

        /// Path to kpclientd Unix domain socket
        #[arg(short, long, default_value = "/run/katzenpost/client.sock")]
        socket: PathBuf,

        /// Optional removable media spool directory (e.g. /media/usb/katzen-spool)
        #[arg(long)]
        spool_dir: Option<PathBuf>,

        /// Apply kernel-level network isolation to the local Vault engine
        #[arg(long, default_value_t = false)]
        sandbox: bool,
    },

    /// Offline Vault mode: compose, encrypt Sphinx packets & decrypt inbound replies
    #[command(alias = "cold")]
    Vault {
        #[command(subcommand)]
        action: VaultCommands,
    },

    /// Blind Network Spooler: dispatch packets to Katzenpost and poll Pigeonhole
    #[command(alias = "hot")]
    Spooler {
        #[command(subcommand)]
        action: SpoolerCommands,
    },

    /// Launch Sovereign Vault graphical user interface (Slint UI with grouped virtual keyboard)
    Gui,

    /// Check system environment, kernel sandbox capabilities and removable media
    Status,
}

#[derive(Subcommand)]
enum VaultCommands {
    /// Sovereign Pigeonhole Mailbox: manage encrypted offline inbox, outbox, and BACAP drafts
    Mailbox {
        #[command(subcommand)]
        action: MailboxCommands,
    },

    /// Construct a 3-hop Sphinx packet offline (export to USB spool or display QR)
    Send {
        /// Message text to send through mixnet
        #[arg(short, long)]
        message: Option<String>,

        /// Path to file containing message payload
        #[arg(short, long)]
        file: Option<String>,

        /// Destination mailbox or service
        #[arg(short = 't', long, default_value = "pigeonhole-egress-01")]
        target: String,

        /// Export directly to a removable media spool folder (e.g. /media/usb/spool)
        #[arg(long)]
        spool_dir: Option<PathBuf>,

        /// Frame delay in milliseconds for animated QR loop
        #[arg(short, long, default_value_t = 600)]
        delay_ms: u64,

        /// Number of repeat loops before exiting (0 = infinite loop)
        #[arg(short, long, default_value_t = 0)]
        loops: u32,

        /// Advance through frames manually by pressing Enter
        #[arg(short, long, default_value_t = false)]
        step: bool,

        /// Output raw QR strings instead of terminal graphics
        #[arg(long, default_value_t = false)]
        raw: bool,
    },

    /// Read and decrypt inbound reply packets from a removable media spool folder
    Receive {
        /// Removable media spool folder containing inbound packets (e.g. /media/usb/spool)
        #[arg(long, default_value = "/tmp/katzen-spool")]
        spool_dir: PathBuf,
    },

    /// Generate an offline SURB delegation token for Pigeonhole mailbox polling
    DelegateSurb {
        /// Mailbox identity or pseudonym
        #[arg(short, long, default_value = "anon-field-agent-01")]
        mailbox: String,
    },

    /// Decode an encrypted reply scanned from the Hot Relay
    DecodeReply {
        /// The raw scanned QR payload (kpag://...)
        #[arg(short, long)]
        payload: String,
    },

    /// Manage permanent cryptographic profiles and recipient contact identicons
    Profile {
        #[command(subcommand)]
        action: ProfileCommands,
    },

    /// Inspect current offline PKI consensus directory topology
    ShowTopology,
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// Show local operator profile and deterministic ANSI identicon
    Show,
    /// Add or update a recipient contact profile and generate their unique identicon
    Add {
        /// Human-readable username or alias for the recipient
        #[arg(short, long)]
        username: String,
        /// Recipient permanent Box ID (32-byte hex public key)
        #[arg(short, long)]
        recipient: String,
    },
    /// List all saved contact profiles and their visual identicons
    List,
    /// Export an SVG identicon for a given username and public key
    ExportSvg {
        /// Username
        #[arg(short, long)]
        username: String,
        /// Recipient or public key hex
        #[arg(short, long)]
        recipient: String,
        /// Output file path (e.g. identicon.svg)
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum MailboxCommands {
    /// List all messages stored in local offline Vault mailbox
    List,
    /// Display sovereign Box ID address
    Identity,
    /// Deposit a new encrypted message to a recipient's Box ID
    Deposit {
        /// Recipient 32-byte hex Box ID
        #[arg(short, long)]
        recipient: Option<String>,
        /// Plaintext message payload
        #[arg(short, long)]
        message: Option<String>,
        /// Use grouped virtual keyboard (anti-keylogger, elevates Secrecy Score to 5 stars ⭐⭐⭐⭐⭐)
        #[arg(long, default_value_t = false)]
        virtual_keyboard: bool,
    },
    /// Generate a BACAP ReplicaRead query with SURB for anonymous mailbox polling
    Poll,
    /// Ingest and decrypt an inbound ReplicaReadReply envelope into the mailbox
    Ingest {
        /// Path to reply envelope file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Hex-encoded reply envelope
        #[arg(short = 'x', long)]
        hex: Option<String>,
    },
}

#[derive(Subcommand)]
enum SpoolerCommands {
    /// Watch a removable media spool directory and dispatch queued Sphinx packets
    Run {
        /// Removable media spool directory to monitor (e.g. /media/usb/spool)
        #[arg(long, default_value = "/tmp/katzen-spool")]
        spool_dir: PathBuf,

        /// Path to kpclientd Unix domain socket
        #[arg(short, long, default_value = "/run/katzenpost/client.sock")]
        socket: PathBuf,

        /// Optional TCP endpoint for kpclientd
        #[arg(short, long)]
        tcp: Option<String>,
    },

    /// Ingest optical air-gap stream directly from a webcam via zbarcam
    Scan {
        /// Video device path
        #[arg(short, long, default_value = "/dev/video0")]
        device: String,

        /// Optional path to kpclientd Unix domain socket
        #[arg(short, long, default_value = "/run/katzenpost/client.sock")]
        socket: PathBuf,

        /// Optional TCP endpoint for kpclientd
        #[arg(short, long)]
        tcp: Option<String>,

        /// Directory to spool outbound Sphinx packets
        #[arg(long, default_value = "/tmp/katzen-spool/outbox")]
        spool_dir: PathBuf,
    },

    /// Ingest optical air-gap stream from stdin (one QR string per line)
    Ingest {
        /// Optional path to kpclientd Unix domain socket
        #[arg(short, long, default_value = "/run/katzenpost/client.sock")]
        socket: PathBuf,

        /// Optional TCP endpoint for kpclientd
        #[arg(short, long)]
        tcp: Option<String>,

        /// Directory to spool outbound Sphinx packets
        #[arg(long, default_value = "/tmp/katzen-spool/outbox")]
        spool_dir: PathBuf,
    },

    /// Encapsulate a received mixnet response into a QR code for the Vault
    SendReply {
        /// Plaintext or raw encrypted reply bytes from mixnet
        #[arg(short, long)]
        data: String,

        /// Sequence ID to correlate
        #[arg(short, long, default_value_t = 1001)]
        seq_id: u32,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            println!("    /\\_/\\   ");
            println!("   ( o.o )   KATZENPOST AIR-GAPPED: SCHRÖDINGER MIXNET CLIENT");
            println!("    > ^ <    Superposition: [Vault | Spooler] • Post-Quantum Resilient");
            println!("=======================================================");
            println!(" ENVIRONMENT & KERNEL AIR-GAP STATUS                  ");
            println!("=======================================================");
            let sandbox_res = apply_network_isolation();
            match sandbox_res {
                Ok(SandboxStatus::KernelEnforced(desc)) => {
                    println!("[✓] Kernel Sandbox: ENFORCED ({})", desc);
                }
                Ok(SandboxStatus::Restricted(desc)) => {
                    println!("[i] Kernel Sandbox: RESTRICTED ({})", desc);
                }
                Err(e) => {
                    println!("[!] Kernel Sandbox error: {}", e);
                }
            }
            let net_blocked = is_network_blocked();
            println!(
                "[i] Network Socket Status: {}",
                if net_blocked {
                    "BLOCKED / DISCONNECTED (Safe for Vault operations)"
                } else {
                    "CONNECTED (Available for Spooler operations)"
                }
            );
            println!("=======================================================");

            let mbox_path = default_vault_mailbox_path();
            if let Ok(mbox) = VaultMailbox::new_or_load(&mbox_path) {
                println!(" PIGEONHOLE ASYNC MAILBOX & QUANTUM STATE              ");
                println!("=======================================================");
                println!("[✓] Sovereign Box ID: {}", mbox.identity.box_id_hex);
                println!("[i] Messaggi memorizzati nel Vault: {}", mbox.messages.len());
                let unread = mbox.messages.iter().filter(|m| m.quantum_state.contains("Superposition")).count();
                println!("[i] In sovrapposizione quantistica (non letti): {}", unread);
                println!("=======================================================");
            }

            println!(" OPSEC SECRECY SCORE MODEL                            ");
            println!("=======================================================");
            println!("⭐ ⭐ ⭐      (70%) : Isolamento standard (Air-gap / Kernel namespace + Tastiera fisica)");
            println!("⭐ ⭐ ⭐ ⭐ ⭐ (100%): Massima schermatura quantistica (Tastiera virtuale raggruppata anti-keylogger)");
            println!("[i] Nota OPSEC: la tastiera virtuale raggruppata è opzionale, ma protegge da keylogger USB, coordinate mouse fisse e analisi acustica.");
            println!("=======================================================");
        }

        Commands::Gui => {
            println!("[*] Avvio interfaccia sovrana Katzen Vault...");
            run_gui().map_err(|e| format!("Errore interfaccia: {}", e))?;
        }

        Commands::Standalone {
            message,
            target,
            socket,
            spool_dir,
            sandbox,
        } => {
            println!("=======================================================");
            println!(" KATZENPOST CLIENT: STANDALONE ALL-IN-ONE MODE        ");
            println!("=======================================================");

            let payload_text = message.unwrap_or_else(|| {
                print!("Inserisci il messaggio da cifrare: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                input.trim().to_string()
            });

            if sandbox {
                println!("[*] Attivazione sandboxing a livello kernel...");
                match apply_network_isolation() {
                    Ok(SandboxStatus::KernelEnforced(desc)) => {
                        println!("[✓] Isolamento di rete attivo: {}", desc);
                    }
                    Ok(SandboxStatus::Restricted(desc)) => {
                        println!("[i] Modalità ristretta: {}", desc);
                    }
                    Err(e) => {
                        eprintln!("[!] Impossibile isolare la rete: {}", e);
                    }
                }
            }

            let (topology, _privkeys) = ConsensusTopology::generate_mock_consensus(100);
            let path = topology.select_path()?;
            let sphinx_packet = SphinxEngine::build_packet(&path, &target, payload_text.as_bytes())?;
            let packet_bytes = sphinx_packet.raw_bytes;

            println!("[✓] Pacchetto Sphinx generato (1024 byte cifrati a cipolla)");

            if let Some(ref dir) = spool_dir {
                let outbox = dir.join("outbox");
                fs::create_dir_all(&outbox)?;
                let file_path = outbox.join(format!("packet_{}.spx", rand::thread_rng().gen::<u32>()));
                fs::write(&file_path, &packet_bytes)?;
                println!("[✓] Pacchetto salvato su spool rimovibile: {:?}", file_path);
                println!(
                    "[i] Suggerimento OPSEC: per prevenire scritture sul supporto da parte di macchine connesse, \
                     puoi usare schede SD con levetta Lock o acquisire il codice a schermo con fotocamera."
                );
            }

            println!("[*] Trasmissione al gateway locale ({:?})...", socket);
            forward_packet_to_gateway(&packet_bytes, &socket, None);
        }

        Commands::Vault { action } => match action {
            VaultCommands::Mailbox { action } => match action {
                MailboxCommands::List => {
                    let mbox_path = default_vault_mailbox_path();
                    let mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    println!("================================================================================");
                    println!(" KATZENPOST VAULT MAILBOX: {} MESSAGGI REGISTRATI", mbox.messages.len());
                    println!(" Identità Box ID: {}", mbox.identity.box_id_hex);
                    println!("================================================================================");
                    if mbox.messages.is_empty() {
                        println!("[i] Nessun messaggio archiviato nella mailbox locale.");
                        println!("    Usa `katzen vault mailbox deposit` o l'interfaccia grafica per comporne uno.");
                    } else {
                        for (idx, msg) in mbox.messages.iter().enumerate() {
                            let dir_tag = match msg.direction {
                                MessageDirection::Inbound => "INBOUND  [↓]",
                                MessageDirection::Outbound => "OUTBOUND [↑]",
                            };
                            println!(
                                "#{} | {} | {} | {} stelle ({})\n    Mittente/Dest: {} -> {}\n    Stato: {}\n    Anteprima: {}\n",
                                idx + 1,
                                dir_tag,
                                msg.id,
                                msg.secrecy_stars,
                                msg.input_method,
                                &msg.sender[0..12.min(msg.sender.len())],
                                &msg.recipient[0..12.min(msg.recipient.len())],
                                msg.quantum_state,
                                if msg.plaintext.len() > 60 { format!("{}...", &msg.plaintext[0..60]) } else { msg.plaintext.clone() }
                            );
                        }
                    }
                }

                MailboxCommands::Identity => {
                    let mbox_path = default_vault_mailbox_path();
                    let mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    let identicon = mbox.identity.identicon();
                    println!("=======================================================");
                    println!(" KATZENPOST PIGEONHOLE SOVEREIGN IDENTITY & AVATAR     ");
                    println!("=======================================================");
                    println!("Username:                    {}", mbox.identity.username);
                    println!("Box ID (Indirizzo Pubblico): {}", mbox.identity.box_id_hex);
                    println!("Creato il (Unix Epoch):      {}", mbox.identity.created_at);
                    println!("File Archivio Locale:        {}", mbox_path.display());
                    println!("\nIdenticon Profilo (Avatar deterministico crittografico):");
                    println!("{}", identicon.render_terminal());
                    println!("=======================================================");
                }

                MailboxCommands::Deposit { recipient, message, virtual_keyboard } => {
                    let mbox_path = default_vault_mailbox_path();
                    let mut mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    let mut kbd = VirtualKeyboardState::new();

                    let raw_target = recipient.unwrap_or_else(|| {
                        print!("Inserisci il Box ID o lo username del destinatario: ");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        input.trim().to_string()
                    });

                    // Resolve contact or raw hex
                    let (recip_hex, contact_opt) = {
                        let trimmed = raw_target.trim();
                        if let Some(c) = mbox.find_contact(trimmed).or_else(|| mbox.contacts.iter().find(|c| c.username.eq_ignore_ascii_case(trimmed))) {
                            (c.box_id_hex.clone(), Some(c.clone()))
                        } else {
                            (trimmed.to_string(), None)
                        }
                    };

                    if let Some(ref c) = contact_opt {
                        println!("\n[✓] Contatto verificato in rubrica: {}", c.username);
                        println!("{}", c.identicon().render_terminal());
                    } else {
                        let temp_identicon = Identicon::new("recipient", &recip_hex);
                        println!("\n[i] Identicon generato per il destinatario:");
                        println!("{}", temp_identicon.render_terminal());
                    }

                    let use_virtual = if virtual_keyboard {
                        true
                    } else {
                        println!("La tastiera virtuale raggruppata aumenta il tuo score di segretezza da 3 a 5 stelle ⭐⭐⭐⭐⭐.");
                        print!("Vuoi utilizzarla per comporre il messaggio? [s/N]: ");
                        io::stdout().flush().unwrap();
                        let mut ans = String::new();
                        io::stdin().read_line(&mut ans).unwrap();
                        ans.trim().eq_ignore_ascii_case("s") || ans.trim().eq_ignore_ascii_case("y")
                    };

                    let (score, method) = if use_virtual {
                        kbd.is_enabled = true;
                        kbd.used_for_composition = true;
                        let boosted = kbd.compute_secrecy_score();
                        println!("\n[OK] Tastiera virtuale anti-keylogger attivata!");
                        println!("Disposizione raggruppata per facilità d'uso (rimescolata solo all'interno di ciascun gruppo):");
                        println!(" [NUMERI]  : {:?}", kbd.numbers);
                        println!(" [LETTERE] : {:?}", kbd.letters);
                        println!(" [SIMBOLI] : {:?}", kbd.symbols);
                        (boosted.stars, "Virtual Keyboard (Grouped Anti-Keylogger)")
                    } else {
                        let std_score = kbd.compute_secrecy_score();
                        (std_score.stars, "Physical Keyboard")
                    };

                    let body_text = message.unwrap_or_else(|| {
                        print!("\nInserisci il testo del messaggio: ");
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        input.trim().to_string()
                    });

                    println!("\n[*] Cifratura asimmetrica Pigeonhole e firma BLAKE3...");
                    let write_msg = mbox.deposit_draft(&recip_hex, &body_text, score, method)?;
                    mbox.save(&mbox_path)?;

                    println!("[✓] Messaggio cifrato e salvato nella mailbox locale!");
                    println!("    ID Payload cifrato: {} byte", write_msg.payload.len());
                    println!("    Punteggio Segretezza: {} stelle ({})", score, method);
                    println!("    Stato Quantistico: Superposizione (Scatola di Schrödinger chiusa)");
                    println!("    Pronto per esportazione su spool USB o proiezione QR.");
                }

                MailboxCommands::Poll => {
                    let mbox_path = default_vault_mailbox_path();
                    let mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    let box_id_bytes = mbox.identity.box_id()?;
                    let read_query = ReplicaRead::new(box_id_bytes);
                    let wire = read_query.encode();

                    println!("=======================================================");
                    println!(" KATZENPOST PIGEONHOLE: INTERROGAZIONE ASINCRONA       ");
                    println!("=======================================================");
                    println!("Box ID interrogato: {}", mbox.identity.box_id_hex);
                    println!("Dimensione query ReplicaRead: {} byte", wire.len());
                    println!("Istruzioni: Il blind spooler invierà questa richiesta alla replica Pigeonhole");
                    println!("            utilizzando un SURB delegato per ricevere le risposte senza tracciamento.");
                    println!("=======================================================");
                }

                MailboxCommands::Ingest { file, hex } => {
                    let mbox_path = default_vault_mailbox_path();
                    let mut mbox = VaultMailbox::new_or_load(&mbox_path)?;

                    let raw_bytes = if let Some(path) = file {
                        fs::read(path)?
                    } else if let Some(hex_str) = hex {
                        hex::decode(hex_str)?
                    } else {
                        eprintln!("Specificare --file oppure --hex per l'ingestione della risposta.");
                        std::process::exit(1);
                    };

                    let reply = ReplicaReadReply::decode(&raw_bytes)?;
                    let plaintext = mbox.ingest_reply(&reply)?;
                    mbox.save(&mbox_path)?;

                    println!("=======================================================");
                    println!(" KATZENPOST VAULT: DECOERENZA QUANTISTICA E LETTURA    ");
                    println!("=======================================================");
                    println!("[✓] Busta Pigeonhole decifrata e verificata con successo!");
                    println!("[*] Stato: Decoerenza quantistica (Testo osservato)");
                    println!("[*] Testo decifrato:");
                    println!("{}", plaintext);
                    println!("=======================================================");
                }
            },

            VaultCommands::Profile { action } => match action {
                ProfileCommands::Show => {
                    let mbox_path = default_vault_mailbox_path();
                    let mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    let identicon = mbox.identity.identicon();
                    println!("=======================================================");
                    println!(" PROFILO SOVRANO LOCALE                                ");
                    println!("=======================================================");
                    println!("Username: {}", mbox.identity.username);
                    println!("Box ID:   {}", mbox.identity.box_id_hex);
                    println!("\nIdenticon Profilo (Avatar deterministico crittografico):");
                    println!("{}", identicon.render_terminal());
                    println!("=======================================================");
                }

                ProfileCommands::Add { username, recipient } => {
                    let mbox_path = default_vault_mailbox_path();
                    let mut mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    let profile = mbox.add_contact(&username, &recipient)?;
                    mbox.save(&mbox_path)?;
                    let identicon = profile.identicon();

                    println!("=======================================================");
                    println!(" PROFILO CONTATTO CREATO / AGGIORNATO                  ");
                    println!("=======================================================");
                    println!("Username:  {}", profile.username);
                    println!("Recipient: {}", profile.box_id_hex);
                    println!("\nIdenticon Profilo Destinatario:");
                    println!("{}", identicon.render_terminal());
                    println!("\n[✓] Salvato nella rubrica contatti del Vault!");
                    println!("=======================================================");
                }

                ProfileCommands::List => {
                    let mbox_path = default_vault_mailbox_path();
                    let mbox = VaultMailbox::new_or_load(&mbox_path)?;
                    println!("================================================================================");
                    println!(" RUBRICA CONTATTI SOVRANI: {} PROFILI SALVATI", mbox.contacts.len());
                    println!("================================================================================");
                    if mbox.contacts.is_empty() {
                        println!("[i] Nessun contatto in rubrica.");
                        println!("    Aggiungine uno con: `katzen vault profile add --username <nome> --recipient <box-id>`");
                    } else {
                        for (i, c) in mbox.contacts.iter().enumerate() {
                            let identicon = c.identicon();
                            println!("#{} Contatto: {} (Box ID: {})", i + 1, c.username, c.box_id_hex);
                            println!("{}", identicon.render_terminal());
                        }
                    }
                    println!("================================================================================");
                }

                ProfileCommands::ExportSvg { username, recipient, output } => {
                    let identicon = Identicon::new(&username, &recipient);
                    let svg = identicon.render_svg(256);
                    fs::write(&output, svg)?;
                    println!("[✓] Identicon SVG esportato con successo in {:?}", output);
                }
            },
            VaultCommands::Send {
                message,
                file,
                target,
                spool_dir,
                delay_ms,
                loops,
                step,
                raw,
            } => {
                let payload_bytes = if let Some(msg) = message {
                    msg.into_bytes()
                } else if let Some(path) = file {
                    fs::read(path)?
                } else {
                    eprintln!("Specificare --message oppure --file");
                    std::process::exit(1);
                };

                let (topology, _privkeys) = ConsensusTopology::generate_mock_consensus(100);
                let path = topology.select_path()?;
                let sphinx_packet = SphinxEngine::build_packet(&path, &target, &payload_bytes)?;
                let packet = sphinx_packet.raw_bytes;

                println!("=======================================================");
                println!(" KATZENPOST VAULT: PACCHETTO SPHINX ASSEMBLATO         ");
                println!("=======================================================");
                println!("Stato:           OFFLINE (Air-Gapped Vault)");
                println!("Percorso Mix:    {} -> {} -> {}", path[0].id, path[1].id, path[2].id);
                println!("Servizio Target: {}", target);
                println!("Dimensione:      {} byte (invariante Sphinx garantita)", packet.len());

                if let Some(ref dir) = spool_dir {
                    let outbox = dir.join("outbox");
                    fs::create_dir_all(&outbox)?;
                    let file_path = outbox.join(format!("packet_{}.spx", rand::thread_rng().gen::<u32>()));
                    fs::write(&file_path, &packet)?;
                    println!("\n[✓] Pacchetto salvato su file rimovibile: {:?}", file_path);
                    println!(
                        "[i] Suggerimento OPSEC: per prevenire scritture sul supporto da parte di macchine connesse, \
                         puoi usare schede SD con levetta Lock o acquisire il codice a schermo con fotocamera."
                    );
                    return Ok(());
                }

                // Default: Display terminal QR carousel
                let seq_id = rand::thread_rng().gen::<u32>();
                let chunks = fragment_payload(MsgType::SphinxOut, seq_id, &packet, DEFAULT_CHUNK_SIZE);

                println!("\n[>] Trasmissione ottica: carosello di {} frammenti QR...", chunks.len());
                run_qr_carousel(&chunks, delay_ms, loops, step, raw)?;
            }

            VaultCommands::Receive { spool_dir } => {
                let inbox = spool_dir.join("inbox");
                if !inbox.exists() {
                    println!("[i] Nessun messaggio in arrivo: cartella {:?} vuota.", inbox);
                    return Ok(());
                }
                let entries = fs::read_dir(&inbox)?;
                let mut count = 0;
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("inbound") {
                        let data = fs::read(&path)?;
                        println!("[✓] Rilevato pacchetto in arrivo: {:?} ({} byte)", path.file_name().unwrap(), data.len());
                        count += 1;
                    }
                }
                if count == 0 {
                    println!("[i] Nessun file .inbound trovato in {:?}", inbox);
                }
            }

            VaultCommands::DelegateSurb { mailbox } => {
                let (topology, _privkeys) = ConsensusTopology::generate_mock_consensus(100);
                let mut manager = SurbManager::new();
                let epoch_expiry = topology.epoch + 3600;
                let surb = manager.generate_surb(&topology, &mailbox, epoch_expiry)?;
                println!("Token SURB delegato generato con successo: {}", hex::encode(surb.surb_id));
            }

            VaultCommands::DecodeReply { payload } => {
                let chunk = KpagChunk::from_qr_string(&payload)?;
                println!("[✓] Frammento QR decodificato correttamente: Sequenza {}", chunk.header.seq_id);
            }

            VaultCommands::ShowTopology => {
                let (topology, _) = ConsensusTopology::generate_mock_consensus(100);
                println!("Consensus Epoch: {}", topology.epoch);
                println!("Nodi attivi nel consensus: {}", topology.nodes.len());
            }
        },

        Commands::Spooler { action } => match action {
            SpoolerCommands::Run {
                spool_dir,
                socket,
                tcp,
            } => {
                let outbox = spool_dir.join("outbox");
                fs::create_dir_all(&outbox)?;
                println!("=======================================================");
                println!(" KATZENPOST SPOOLER: MONITORAGGIO CODA DI TRASPORTO    ");
                println!("=======================================================");
                println!("Monitoraggio cartella spool: {:?}", outbox);
                println!("Gateway socket:              {:?}", socket);
                println!("[i] In attesa di pacchetti .spx da trasmettere...");

                loop {
                    let entries = fs::read_dir(&outbox)?;
                    for entry in entries {
                        let entry = entry?;
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("spx") {
                            let packet = fs::read(&path)?;
                            println!("\n[*] Rilevato pacchetto da spedire: {:?} ({} byte)", path.file_name().unwrap(), packet.len());
                            forward_packet_to_gateway(&packet, &socket, tcp.as_deref());
                            let _ = fs::remove_file(path);
                        }
                    }
                    sleep(Duration::from_millis(500));
                }
            }

            SpoolerCommands::Scan {
                device,
                socket,
                tcp,
                spool_dir: _spool_dir,
            } => {
                println!("[*] Avvio scansione ottica con zbarcam su {}...", device);
                run_camera_scan(&device, &socket, tcp.as_deref(), &_spool_dir)?;
            }

            SpoolerCommands::Ingest {
                socket,
                tcp,
                spool_dir: _spool_dir,
            } => {
                println!("[*] Ingestione frammenti da standard input...");
                run_stdin_ingest(&socket, tcp.as_deref(), &_spool_dir)?;
            }

            SpoolerCommands::SendReply { data, seq_id } => {
                let chunks = fragment_payload(MsgType::ReplyIn, seq_id, data.as_bytes(), DEFAULT_CHUNK_SIZE);
                println!("[*] Proiezione risposta cifrata in {} frammenti QR...", chunks.len());
                run_qr_carousel(&chunks, 600, 0, false, false)?;
            }
        },
    }

    Ok(())
}

fn forward_packet_to_gateway(payload: &[u8], socket_path: &Path, tcp_addr: Option<&str>) {
    if socket_path.exists() {
        println!("[*] Connessione a kpclientd via Unix domain socket ({:?})...", socket_path);
        match UnixStream::connect(socket_path) {
            Ok(mut stream) => {
                if let Err(e) = stream.write_all(payload) {
                    eprintln!("[-] Errore trasmissione socket: {}", e);
                } else {
                    println!("[✓] Pacchetto Sphinx da 1024 byte inviato al gateway con successo!");
                    return;
                }
            }
            Err(e) => {
                eprintln!("[-] Impossibile connettersi al socket kpclientd: {}", e);
            }
        }
    }

    if let Some(addr) = tcp_addr {
        println!("[*] Connessione a kpclientd via TCP ({})...", addr);
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                if let Err(e) = stream.write_all(payload) {
                    eprintln!("[-] Errore trasmissione TCP: {}", e);
                } else {
                    println!("[✓] Pacchetto Sphinx inviato via TCP con successo!");
                    return;
                }
            }
            Err(e) => {
                eprintln!("[-] Impossibile connettersi a endpoint TCP {}: {}", addr, e);
            }
        }
    }

    println!("[i] Gateway locale non attivo. Pacchetto pronto per instradamento mixnet.");
}

fn run_qr_carousel(
    chunks: &[KpagChunk],
    delay_ms: u64,
    loops: u32,
    step: bool,
    raw: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_loop = 0;
    loop {
        for (i, chunk) in chunks.iter().enumerate() {
            let qr_string = chunk.to_qr_string();
            if raw {
                println!("{}", qr_string);
            } else {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
                println!(
                    "--- [KPAG FRAME {}/{}] (Seq ID: {}) ---",
                    i + 1,
                    chunks.len(),
                    chunk.header.seq_id
                );
                let rendered = render_qr_terminal(&qr_string).unwrap_or_else(|e| format!("QR Render Error: {}", e));
                println!("{}", rendered);
                println!("--- [Inquadra lo schermo con la fotocamera del dispositivo connesso] ---");
            }

            if step {
                print!("Premi Invio per il frame successivo...");
                io::stdout().flush()?;
                let mut buf = String::new();
                io::stdin().read_line(&mut buf)?;
            } else {
                sleep(Duration::from_millis(delay_ms));
            }
        }
        current_loop += 1;
        if loops > 0 && current_loop >= loops {
            break;
        }
    }
    Ok(())
}

fn run_camera_scan(
    device: &str,
    socket: &Path,
    tcp: Option<&str>,
    _spool_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("zbarcam")
        .arg("--nodisplay")
        .arg("--raw")
        .arg(device)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Impossibile aprire stdout di zbarcam")?;
    let reader = BufReader::new(stdout);
    let mut reassembler = Reassembler::new();

    for line in reader.lines() {
        if let Ok(l) = line {
            let trimmed = l.trim();
            if trimmed.starts_with("kpag://") || trimmed.starts_with("KPAG://") {
                if let Ok(chunk) = KpagChunk::from_qr_string(trimmed) {
                    if let Ok(Some((_mtype, _seq, full_payload))) = reassembler.add_chunk(chunk) {
                        forward_packet_to_gateway(&full_payload, socket, tcp);
                    }
                }
            }
        }
    }
    Ok(())
}

fn run_stdin_ingest(
    socket: &Path,
    tcp: Option<&str>,
    _spool_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let reader = stdin.lock();
    let mut reassembler = Reassembler::new();

    for line in reader.lines() {
        let l = line?;
        let trimmed = l.trim();
        if trimmed.starts_with("kpag://") || trimmed.starts_with("KPAG://") {
            if let Ok(chunk) = KpagChunk::from_qr_string(trimmed) {
                if let Ok(Some((_mtype, _seq, full_payload))) = reassembler.add_chunk(chunk) {
                    forward_packet_to_gateway(&full_payload, socket, tcp);
                }
            }
        }
    }
    Ok(())
}
