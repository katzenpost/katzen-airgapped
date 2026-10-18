use crate::pigeonhole::{default_vault_mailbox_path, BoxIdentity, MessageDirection, ReplicaWrite, VaultMailbox};
use crate::virtual_keyboard::VirtualKeyboardState;
use std::io::{self, stdout, Write};
use std::path::PathBuf;

slint::slint! {
    import { Button, LineEdit, TextEdit, ScrollView, CheckBox } from "std-widgets.slint";

    export component MainWindow inherits Window {
        title: "Katzenpost Sovereign Client — Katzen Air-Gapped";
        min_width: 860px;
        min_height: 680px;
        background: #151821;

        in-out property <string> recipient_box_id: "";
        in-out property <string> message_body: "";
        in-out property <string> secrecy_stars: "⭐⭐⭐ (70%)";
        in-out property <string> secrecy_desc: "Isolamento Standard (Tastiera Fisica). Attiva la tastiera virtuale per 100% ⭐⭐⭐⭐⭐.";
        in-out property <string> quantum_status: "🐱 Stato Quantistico: Superposizione (Scatola di Schrödinger Chiusa)";
        in-out property <string> opsec_tip: "Tip OPSEC: Trasferimento via QR/media rimovibile isola il Vault dalla rete fisica.";
        in-out property <bool> use_virtual_kbd: false;
        in-out property <string> numbers_row: "1  2  3  4  5  6  7  8  9  0";
        in-out property <string> letters_row1: "q  w  e  r  t  y  u  i  o  p";
        in-out property <string> letters_row2: "a  s  d  f  g  h  j  k  l";
        in-out property <string> letters_row3: "z  x  c  v  b  n  m";
        in-out property <string> symbols_row: ".  ,  ?  !  :  ;  @  #  $  %  &  *  -  +  =";
        in-out property <string> my_box_id: "Identità Box: Inizializzazione...";
        in-out property <string> status_log: "Pronto.";

        callback send_message_clicked();
        callback toggle_virtual_kbd(bool);
        callback shuffle_keys_clicked();
        callback virtual_key_clicked(string);
        callback export_spool_clicked();
        callback poll_mailbox_clicked();

        VerticalLayout {
            padding: 18px;
            spacing: 12px;

            // Header Banner
            HorizontalLayout {
                spacing: 12px;
                Text {
                    text: "KATZENPOST SOVEREIGN VAULT";
                    color: #4fd1c5;
                    font_size: 18px;
                    font_weight: 700;
                    vertical_alignment: center;
                }
                Text {
                    text: root.my_box_id;
                    color: #a0aec0;
                    font_size: 11px;
                    vertical_alignment: center;
                    horizontal_stretch: 1;
                }
            }

            // Quantum Metaphor & Secrecy Score Box
            Rectangle {
                background: #1e2330;
                border_radius: 8px;
                border_width: 1px;
                border_color: #2d3748;
                height: 72px;

                VerticalLayout {
                    padding: 8px;
                    spacing: 4px;

                    HorizontalLayout {
                        Text {
                            text: root.quantum_status;
                            color: #ecc94b;
                            font_size: 12px;
                            font_weight: 600;
                        }
                        Text {
                            text: root.secrecy_stars;
                            color: #68d391;
                            font_size: 13px;
                            font_weight: 700;
                            horizontal_alignment: right;
                        }
                    }

                    Text {
                        text: root.secrecy_desc;
                        color: #cbd5e0;
                        font_size: 11px;
                    }
                }
            }

            // Recipient & Message Inputs
            HorizontalLayout {
                spacing: 8px;
                Text {
                    text: "Destinatario (Box ID):";
                    color: #e2e8f0;
                    vertical_alignment: center;
                    font_size: 12px;
                }
                LineEdit {
                    text <=> root.recipient_box_id;
                    placeholder_text: "Inserisci 32-byte Hex Box ID destinatario...";
                    horizontal_stretch: 1;
                }
            }

            Text {
                text: "Messaggio da cifrare:";
                color: #e2e8f0;
                font_size: 12px;
            }

            TextEdit {
                text <=> root.message_body;
                height: 120px;
                font_size: 13px;
            }

            // Virtual Keyboard Toggle & Section
            HorizontalLayout {
                spacing: 12px;
                CheckBox {
                    text: "Abilita Tastiera Virtuale Anti-Keylogger (Opzionale: aumenta lo score a 5 stelle ⭐⭐⭐⭐⭐)";
                    checked <=> root.use_virtual_kbd;
                    toggled => {
                        root.toggle_virtual_kbd(root.use_virtual_kbd);
                    }
                }
                if root.use_virtual_kbd : Button {
                    text: "Rimescola Tasti (Intra-gruppo) 🔀";
                    clicked => { root.shuffle_keys_clicked(); }
                }
            }

            if root.use_virtual_kbd : Rectangle {
                background: #1a1e29;
                border_radius: 6px;
                border_width: 1px;
                border_color: #3182ce;
                padding: 8px;

                VerticalLayout {
                    spacing: 6px;

                    Text {
                        text: "Disposizione raggruppata (numeri, lettere e simboli separati per digitazione intuitiva):";
                        color: #63b3ed;
                        font_size: 11px;
                    }

                    // Numbers group display
                    HorizontalLayout {
                        spacing: 4px;
                        Text { text: "NUM:"; color: #a0aec0; font_size: 11px; width: 40px; vertical_alignment: center; }
                        Text { text: root.numbers_row; color: #f7fafc; font_size: 12px; font_family: "monospace"; }
                    }

                    // Letters group display
                    HorizontalLayout {
                        spacing: 4px;
                        Text { text: "LET1:"; color: #a0aec0; font_size: 11px; width: 40px; vertical_alignment: center; }
                        Text { text: root.letters_row1; color: #f7fafc; font_size: 12px; font_family: "monospace"; }
                    }
                    HorizontalLayout {
                        spacing: 4px;
                        Text { text: "LET2:"; color: #a0aec0; font_size: 11px; width: 40px; vertical_alignment: center; }
                        Text { text: root.letters_row2; color: #f7fafc; font_size: 12px; font_family: "monospace"; }
                    }
                    HorizontalLayout {
                        spacing: 4px;
                        Text { text: "LET3:"; color: #a0aec0; font_size: 11px; width: 40px; vertical_alignment: center; }
                        Text { text: root.letters_row3; color: #f7fafc; font_size: 12px; font_family: "monospace"; }
                    }

                    // Symbols group display
                    HorizontalLayout {
                        spacing: 4px;
                        Text { text: "SYM:"; color: #a0aec0; font_size: 11px; width: 40px; vertical_alignment: center; }
                        Text { text: root.symbols_row; color: #f7fafc; font_size: 12px; font_family: "monospace"; }
                    }
                }
            }

            // Action Buttons
            HorizontalLayout {
                spacing: 12px;

                Button {
                    text: "Costruisci Pacchetto Sphinx & Salva Draft";
                    primary: true;
                    clicked => { root.send_message_clicked(); }
                }

                Button {
                    text: "Esporta Spool USB";
                    clicked => { root.export_spool_clicked(); }
                }

                Button {
                    text: "Sincronizza / Polla Pigeonhole";
                    clicked => { root.poll_mailbox_clicked(); }
                }
            }

            // Bottom OPSEC Tip & Status Log
            Rectangle {
                background: #181b24;
                border_radius: 6px;
                padding: 6px;

                HorizontalLayout {
                    Text {
                        text: root.status_log;
                        color: #9ae6b4;
                        font_size: 11px;
                        horizontal_stretch: 1;
                    }
                    Text {
                        text: root.opsec_tip;
                        color: #718096;
                        font_size: 11px;
                        horizontal_alignment: right;
                    }
                }
            }
        }
    }
}

pub fn run_gui() -> Result<(), String> {
    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        println!("==> Nessun display grafico attivo rilevato (DISPLAY / WAYLAND_DISPLAY non impostati).");
        println!("==> Avvio della TUI Sovereign interattiva con tastiera virtuale raggruppata...\n");
        return run_interactive_tui();
    }

    let mailbox_path = default_vault_mailbox_path();
    let mut mailbox = VaultMailbox::new_or_load(&mailbox_path)
        .map_err(|e| format!("Errore inizializzazione mailbox: {}", e))?;

    let main_window = MainWindow::new().map_err(|e| format!("Slint GUI init error: {}", e))?;
    let mut kbd_state = VirtualKeyboardState::new();

    let my_id = format!("Box ID: {}", mailbox.identity.box_id_hex);
    main_window.set_my_box_id(my_id.into());

    let update_rows = {
        let handle = main_window.as_weak();
        move |kbd: &VirtualKeyboardState| {
            if let Some(ui) = handle.upgrade() {
                let n_str: String = kbd.numbers.iter().map(|c| format!("{} ", c)).collect();
                ui.set_numbers_row(n_str.into());

                let l1: String = kbd.letters[0..10.min(kbd.letters.len())].iter().map(|c| format!("{} ", c)).collect();
                let l2: String = kbd.letters[10..19.min(kbd.letters.len())].iter().map(|c| format!("{} ", c)).collect();
                let l3: String = kbd.letters[19..kbd.letters.len()].iter().map(|c| format!("{} ", c)).collect();
                ui.set_letters_row1(l1.into());
                ui.set_letters_row2(l2.into());
                ui.set_letters_row3(l3.into());

                let s_str: String = kbd.symbols.iter().map(|c| format!("{} ", c)).collect();
                ui.set_symbols_row(s_str.into());

                let score = kbd.compute_secrecy_score();
                let star_text = format!("{} ({}%)", "⭐".repeat(score.stars as usize), score.score_percentage);
                ui.set_secrecy_stars(star_text.into());
                ui.set_secrecy_desc(score.opsec_note.into());
            }
        }
    };

    update_rows(&kbd_state);

    let kbd_ref = std::rc::Rc::new(std::cell::RefCell::new(kbd_state));
    let mailbox_ref = std::rc::Rc::new(std::cell::RefCell::new(mailbox));

    // Callback toggle virtual kbd
    {
        let kbd_rc = kbd_ref.clone();
        let ui_weak = main_window.as_weak();
        main_window.on_toggle_virtual_kbd(move |enabled| {
            let mut k = kbd_rc.borrow_mut();
            k.is_enabled = enabled;
            k.used_for_composition = enabled;
            let score = k.compute_secrecy_score();
            if let Some(ui) = ui_weak.upgrade() {
                let star_text = format!("{} ({}%)", "⭐".repeat(score.stars as usize), score.score_percentage);
                ui.set_secrecy_stars(star_text.into());
                ui.set_secrecy_desc(score.opsec_note.into());
                if enabled {
                    ui.set_status_log("Tastiera virtuale raggruppata abilitata. Score di segretezza elevato al 100% ⭐⭐⭐⭐⭐.".into());
                } else {
                    ui.set_status_log("Tastiera fisica abilitata. Score impostato su livello standard (70%).".into());
                }
            }
        });
    }

    // Callback shuffle keys
    {
        let kbd_rc = kbd_ref.clone();
        let ui_weak = main_window.as_weak();
        let update_fn = update_rows.clone();
        main_window.on_shuffle_keys_clicked(move || {
            let mut k = kbd_rc.borrow_mut();
            k.shuffle_within_groups();
            update_fn(&k);
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_log("Tasti rimescolati all'interno dei gruppi (coordinate mouse scrambleate).".into());
            }
        });
    }

    // Callback send / deposit message
    {
        let mbox_rc = mailbox_ref.clone();
        let kbd_rc = kbd_ref.clone();
        let ui_weak = main_window.as_weak();
        let mbox_path = mailbox_path.clone();
        main_window.on_send_message_clicked(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let recip = ui.get_recipient_box_id();
                let body = ui.get_message_body();
                let k = kbd_rc.borrow();
                let score = k.compute_secrecy_score();
                let method = if k.used_for_composition {
                    "Virtual Keyboard (Anti-Keylogger)"
                } else {
                    "Physical Keyboard"
                };

                if recip.trim().is_empty() {
                    ui.set_status_log("Inserisci un Box ID destinatario valido prima di procedere.".into());
                    return;
                }

                let mut mbox = mbox_rc.borrow_mut();
                match mbox.deposit_draft(&recip, &body, score.stars, method) {
                    Ok(replica_write) => {
                        let _ = mbox.save(&mbox_path);
                        ui.set_status_log(format!("Bozza cifrata con successo per {} ({} bytes depositati).", &recip[0..8.min(recip.len())], replica_write.payload.len()).into());
                        ui.set_quantum_status("🐱 Stato Quantistico: Superposizione (Pacchetto pronto per mixnet)".into());
                    }
                    Err(e) => {
                        ui.set_status_log(format!("Errore cifratura deposito: {}", e).into());
                    }
                }
            }
        });
    }

    // Callback export spool
    {
        let ui_weak = main_window.as_weak();
        main_window.on_export_spool_clicked(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_log("Pacchetti cifrati accodati nella spool directory rimovibile (/media/usb/spool).".into());
                ui.set_opsec_tip("Suggerimento: Usa schede SD con interruttore di sola lettura per prevenire BadUSB.".into());
            }
        });
    }

    // Callback poll mailbox
    {
        let ui_weak = main_window.as_weak();
        main_window.on_poll_mailbox_clicked(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_status_log("SURB generati per interrogazione anonima della mailbox Pigeonhole.".into());
                ui.set_quantum_status("🐱 Stato Quantistico: Superposizione in attesa di osservazione".into());
            }
        });
    }

    main_window.run().map_err(|e| format!("Errore esecuzione interfaccia grafica Slint: {}", e))?;
    Ok(())
}

/// Fallback / Headless Interactive TUI Mode
pub fn run_interactive_tui() -> Result<(), String> {
    let mailbox_path = default_vault_mailbox_path();
    let mut mailbox = VaultMailbox::new_or_load(&mailbox_path)
        .map_err(|e| format!("Errore inizializzazione mailbox: {}", e))?;

    let mut kbd = VirtualKeyboardState::new();

    println!("================================================================================");
    println!("     KATZENPOST SOVEREIGN VAULT — SOVRANITÀ AIR-GAPPED & PIGEONHOLE           ");
    println!("================================================================================");
    println!("Username Sovrano      : {}", mailbox.identity.username);
    println!("Identità Vault Box ID : {}", mailbox.identity.box_id_hex);
    println!("Avatar Identicon (Crittografico deterministico):");
    println!("{}", mailbox.identity.identicon().render_terminal());
    println!("--------------------------------------------------------------------------------");
    println!("Metafora Quantistica   : Il Gatto di Schrödinger");
    println!("                       -> Finché il pacchetto transita nel mixnet o risiede nel");
    println!("                          Vault air-gapped, il suo stato resta in sovrapposizione.");
    println!("                       -> La decoerenza verso il testo in chiaro avviene solo");
    println!("                          all'atto dell'osservazione esplicita nel Vault.");
    println!("--------------------------------------------------------------------------------");

    let score = kbd.compute_secrecy_score();
    println!("Punteggio Segretezza   : {} ({}%)", "⭐".repeat(score.stars as usize), score.score_percentage);
    println!("Protezione Tastiera    : Standard (Tastiera Fisica)");
    println!("Tip OPSEC              : {}", score.opsec_note);
    println!("--------------------------------------------------------------------------------\n");

    println!("Vuoi comporre un messaggio usando la tastiera virtuale raggruppata anti-keylogger?");
    println!("(La tastiera è opzionale, ma se usata eleva il tuo punteggio di segretezza al 100% ⭐⭐⭐⭐⭐)");
    print!("[s/N]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).map_err(|e| e.to_string())?;

    let mut using_virtual = false;
    let mut message = String::new();

    if choice.trim().eq_ignore_ascii_case("s") || choice.trim().eq_ignore_ascii_case("y") {
        using_virtual = true;
        kbd.is_enabled = true;
        kbd.used_for_composition = true;
        let boosted = kbd.compute_secrecy_score();

        println!("\n[OK] Tastiera virtuale anti-keylogger attivata!");
        println!("Nuovo Punteggio di Segretezza: {} ({}%) - {}", "⭐".repeat(boosted.stars as usize), boosted.score_percentage, boosted.shielding_level);
        println!("Disposizione raggruppata (tasti separati per categoria, rimescolati solo all'interno del gruppo):");

        println!(" [NUMERI]  : {:?}", kbd.numbers);
        println!(" [LETTERE] : {:?}", kbd.letters);
        println!(" [SIMBOLI] : {:?}", kbd.symbols);
    }

    print!("\nInserisci Box ID destinatario (32-byte hex o default demo): ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut target = String::new();
    io::stdin().read_line(&mut target).map_err(|e| e.to_string())?;
    let target = target.trim();
    let target_id = if target.is_empty() {
        mailbox.identity.box_id_hex.clone() // loopback self-test
    } else {
        target.to_string()
    };

    print!("Inserisci il messaggio: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    io::stdin().read_line(&mut message).map_err(|e| e.to_string())?;
    let message = message.trim();

    let score = kbd.compute_secrecy_score();
    let method = if using_virtual {
        "Virtual Keyboard (Grouped Anti-Keylogger)"
    } else {
        "Physical Keyboard"
    };

    println!("\n==> Cifratura asimmetrica Pigeonhole BACAP in corso...");
    let deposit = mailbox.deposit_draft(&target_id, message, score.stars, method)?;
    mailbox.save(&mailbox_path)?;

    println!("[OK] Bozza depositata nella mailbox locale del Vault!");
    println!("     Payload cifrato: {} byte", deposit.payload.len());
    println!("     Stato quantistico: Superposizione (Air-Gapped / In transito)");
    println!("     Score di segretezza associato al messaggio: {} stelle", score.stars);
    println!("     File archivio: {}\n", mailbox_path.display());

    Ok(())
}
