use clap::{Parser, Subcommand};
use katzen_airgapped::{fragment_payload, render_qr_terminal, KpagChunk, MsgType, Reassembler, DEFAULT_CHUNK_SIZE};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "katzen-hot")]
#[command(about = "Katzenpost Hot Station (Networked Relay & Blind Forwarder)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest optical air-gap stream from stdin (one QR string per line)
    Ingest {
        /// Optional path to kpclientd Unix domain socket
        #[arg(short, long, default_value = "/run/katzenpost/client.sock")]
        socket: PathBuf,

        /// Optional TCP endpoint for kpclientd
        #[arg(short, long)]
        tcp: Option<String>,

        /// Directory to spool outbound Sphinx packets if daemon is offline
        #[arg(long, default_value = "/tmp/katzen-hot-spool")]
        spool_dir: PathBuf,
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

        /// Directory to spool outbound Sphinx packets if daemon is offline
        #[arg(long, default_value = "/tmp/katzen-hot-spool")]
        spool_dir: PathBuf,
    },

    /// Encapsulate a received mixnet response into a QR code for the Cold Terminal to scan
    SendReply {
        /// Plaintext or raw encrypted reply bytes from mixnet
        #[arg(short, long)]
        data: String,

        /// Sequence ID to correlate
        #[arg(short, long, default_value_t = 1001)]
        seq_id: u32,
    },
}

fn forward_or_spool_packet(
    seq_id: u32,
    msg_type: MsgType,
    payload: &[u8],
    socket_path: &Path,
    tcp_addr: Option<&str>,
    spool_dir: &Path,
) {
    println!("\n=======================================================");
    println!(" [SUCCESS] FULL PACKET REASSEMBLED OVER AIR-GAP!");
    println!("=======================================================");
    println!("Sequence ID:     {}", seq_id);
    println!("Message Type:    {:?}", msg_type);
    println!("Assembled Size:  {} bytes", payload.len());
    println!("Sphinx Invariant: {} (1024 bytes)", if payload.len() == 1024 { "VERIFIED" } else { "CUSTOM" });
    println!("=======================================================");
    println!("[*] Hot Station possesses ZERO encryption keys.");

    // Attempt Unix domain socket forwarding first
    if socket_path.exists() {
        println!("[*] Connecting to kpclientd via Unix domain socket ({:?})...", socket_path);
        match UnixStream::connect(socket_path) {
            Ok(mut stream) => {
                if let Err(e) = stream.write_all(payload) {
                    eprintln!("[-] Error transmitting to kpclientd socket: {}", e);
                } else {
                    println!("[✓] Dispatched 1024-byte Sphinx packet to kpclientd socket successfully!");
                    return;
                }
            }
            Err(e) => {
                eprintln!("[-] Could not connect to kpclientd socket: {}", e);
            }
        }
    }

    // Attempt TCP forwarding if configured
    if let Some(addr) = tcp_addr {
        println!("[*] Connecting to kpclientd via TCP ({addr})...");
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                if let Err(e) = stream.write_all(payload) {
                    eprintln!("[-] Error transmitting to kpclientd TCP: {}", e);
                } else {
                    println!("[✓] Dispatched 1024-byte Sphinx packet to kpclientd TCP successfully!");
                    return;
                }
            }
            Err(e) => {
                eprintln!("[-] Could not connect to kpclientd TCP endpoint: {}", e);
            }
        }
    }

    // Fallback: Spool to local spool directory
    if let Err(e) = fs::create_dir_all(spool_dir) {
        eprintln!("[-] Could not create spool directory: {}", e);
        return;
    }

    let spool_file = spool_dir.join(format!("sphinx_outbound_{seq_id}.bin"));
    match fs::write(&spool_file, payload) {
        Ok(_) => {
            println!("[!] Notice: kpclientd is not running locally.");
            println!("[✓] Sphinx packet spooled to: {}", spool_file.display());
            println!("[*] Ready for transmission when kpclientd daemon starts up.");
        }
        Err(e) => {
            eprintln!("[-] Failed to spool packet: {}", e);
        }
    }
}

fn process_stream<R: BufRead>(
    reader: R,
    socket_path: &Path,
    tcp_addr: Option<&str>,
    spool_dir: &Path,
) {
    let mut reassembler = Reassembler::new();

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match KpagChunk::from_qr_string(trimmed) {
            Ok(chunk) => {
                let seq = chunk.header.seq_id;
                let part = chunk.header.part_index + 1;
                let total = chunk.header.total_parts;

                println!("[+] Validated chunk {}/{} for Seq #{} (BLAKE3 checksum OK)", part, total, seq);

                match reassembler.add_chunk(chunk) {
                    Ok(Some((reconstructed_type, seq_id, payload))) => {
                        forward_or_spool_packet(
                            seq_id,
                            reconstructed_type,
                            &payload,
                            socket_path,
                            tcp_addr,
                            spool_dir,
                        );
                        break;
                    }
                    Ok(None) => {
                        if let Some((current, total)) = reassembler.progress(seq) {
                            println!("    Progress: {}/{} chunks received.", current, total);
                        }
                    }
                    Err(e) => {
                        eprintln!("[-] Reassembly error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[-] Invalid chunk rejected: {}", e);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            socket,
            tcp,
            spool_dir,
        } => {
            println!("=======================================================");
            println!(" KATZENPOST HOT STATION: BLIND RELAY FORWARDER ");
            println!("=======================================================");
            println!("Status: ONLINE (Waiting for optical air-gap stream on stdin)");
            println!("Paste or pipe 'kpag://...' QR tokens line by line.");
            println!("Press Ctrl+D when finished.\n");

            let stdin = io::stdin();
            process_stream(stdin.lock(), &socket, tcp.as_deref(), &spool_dir);
        }

        Commands::Scan {
            device,
            socket,
            tcp,
            spool_dir,
        } => {
            println!("=======================================================");
            println!(" KATZENPOST HOT STATION: WEBCAM LIVE SCANNER ");
            println!("=======================================================");
            println!("Starting zbarcam on device: {}", device);
            println!("Point your webcam at the Cold Terminal QR carousel.\n");

            let mut child = match Command::new("zbarcam")
                .arg("-q")
                .arg("--raw")
                .arg(&device)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[-] Failed to launch zbarcam: {}. Ensure zbarcam is installed.", e);
                    std::process::exit(1);
                }
            };

            let stdout = child.stdout.take().unwrap();
            let reader = BufReader::new(stdout);
            process_stream(reader, &socket, tcp.as_deref(), &spool_dir);

            let _ = child.kill();
        }

        Commands::SendReply { data, seq_id } => {
            let chunks = fragment_payload(
                MsgType::ReplyIn,
                seq_id,
                data.as_bytes(),
                DEFAULT_CHUNK_SIZE,
            );

            println!("=======================================================");
            println!(" KATZENPOST HOT STATION: PRESENTING REPLY TO COLD ");
            println!("=======================================================");
            println!("Presenting encrypted reply QR for Cold Terminal to scan.");

            for (i, chunk) in chunks.iter().enumerate() {
                let qr_str = chunk.to_qr_string();
                let rendered = render_qr_terminal(&qr_str).unwrap();

                println!("\n--- [Frame {}/{}] ---", i + 1, chunks.len());
                println!("{}", rendered);
                println!("Token: {}", qr_str);
            }
        }
    }

    Ok(())
}
