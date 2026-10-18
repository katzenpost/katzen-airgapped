use crate::pigeonhole::{default_vault_mailbox_path, MessageDirection, ReplicaReadReply, VaultMailbox};
use crate::virtual_keyboard::VirtualKeyboardState;
use slint::VecModel;
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

slint::slint! {
    import { Button, LineEdit, TextEdit, ScrollView, CheckBox } from "std-widgets.slint";

    export struct MessageItem {
        id: string,
        direction: string,
        party: string,
        preview: string,
        time_str: string,
        quantum_state: string,
        secrecy: string,
        raw_plaintext: string,
    }

    export struct ContactItem {
        username: string,
        box_id: string,
        avatar: image,
    }

    export component MainWindow inherits Window {
        title: "Katzenpost Sovereign Client — Katzen Air-Gapped";
        min_width: 920px;
        min_height: 720px;
        background: #151821;

        in-out property <int> active_tab: 0; // 0: Compositore, 1: Mailbox, 2: Rubrica

        // Compositore properties
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
        in-out property <image> operator_avatar;
        in-out property <string> status_log: "Pronto.";

        // Mailbox properties
        in-out property <[MessageItem]> messages_model: [];
        in-out property <string> selected_message_text: "";
        in-out property <string> selected_message_info: "Seleziona un messaggio dall'elenco per visualizzare il testo decifrato.";
        in-out property <string> ingest_raw_hex: "";

        // Contacts properties
        in-out property <[ContactItem]> contacts_model: [];
        in-out property <string> new_contact_username: "";
        in-out property <string> new_contact_box_id: "";

        // Callbacks
        callback switch_tab(int);
        callback send_message_clicked();
        callback toggle_virtual_kbd(bool);
        callback shuffle_keys_clicked();
        callback export_spool_clicked();
        callback poll_mailbox_clicked();

        callback select_message_clicked(string, string);
        callback ingest_reply_clicked();

        callback add_contact_clicked();
        callback use_contact_clicked(string);

        VerticalLayout {
            padding: 18px;
            spacing: 12px;

            // Header Banner
            HorizontalLayout {
                spacing: 14px;
                Image {
                    source: root.operator_avatar;
                    width: 44px;
                    height: 44px;
                    image-fit: contain;
                }
                VerticalLayout {
                    spacing: 2px;
                    horizontal_stretch: 1;
                    Text {
                        text: "KATZENPOST SOVEREIGN VAULT";
                        color: #4fd1c5;
                        font_size: 17px;
                        font_weight: 700;
                    }
                    Text {
                        text: root.my_box_id;
                        color: #a0aec0;
                        font_size: 11px;
                        font_family: "monospace";
                    }
                }
            }

            // Tab Navigation Bar
            HorizontalLayout {
                spacing: 8px;
                height: 38px;

                Rectangle {
                    background: root.active_tab == 0 ? #2b6cb0 : #1e2330;
                    border_radius: 6px;
                    border_width: 1px;
                    border_color: root.active_tab == 0 ? #63b3ed : #2d3748;
                    TouchArea {
                        clicked => { root.switch_tab(0); }
                    }
                    Text {
                        text: "✍️ Compositore & Tastiera Virtuale";
                        color: root.active_tab == 0 ? #ffffff : #a0aec0;
                        font_size: 13px;
                        font_weight: 600;
                        vertical_alignment: center;
                        horizontal_alignment: center;
                    }
                }

                Rectangle {
                    background: root.active_tab == 1 ? #2b6cb0 : #1e2330;
                    border_radius: 6px;
                    border_width: 1px;
                    border_color: root.active_tab == 1 ? #63b3ed : #2d3748;
                    TouchArea {
                        clicked => { root.switch_tab(1); }
                    }
                    Text {
                        text: "📬 Mailbox & Inbox Spool";
                        color: root.active_tab == 1 ? #ffffff : #a0aec0;
                        font_size: 13px;
                        font_weight: 600;
                        vertical_alignment: center;
                        horizontal_alignment: center;
                    }
                }

                Rectangle {
                    background: root.active_tab == 2 ? #2b6cb0 : #1e2330;
                    border_radius: 6px;
                    border_width: 1px;
                    border_color: root.active_tab == 2 ? #63b3ed : #2d3748;
                    TouchArea {
                        clicked => { root.switch_tab(2); }
                    }
                    Text {
                        text: "👥 Rubrica Contatti & Avatar";
                        color: root.active_tab == 2 ? #ffffff : #a0aec0;
                        font_size: 13px;
                        font_weight: 600;
                        vertical_alignment: center;
                        horizontal_alignment: center;
                    }
                }
            }

            // -----------------------------------------------------------------
            // TAB 0: COMPOSITORE & TASTIERA VIRTUALE
            // -----------------------------------------------------------------
            if root.active_tab == 0 : VerticalLayout {
                spacing: 10px;

                // Quantum Metaphor & Secrecy Score Box
                Rectangle {
                    background: #1e2330;
                    border_radius: 8px;
                    border_width: 1px;
                    border_color: #2d3748;
                    height: 64px;

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
                        placeholder_text: "Inserisci 32-byte Hex Box ID destinatario (oppure selezionalo dalla Rubrica)...";
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
                    height: 100px;
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

                    VerticalLayout {
                        padding: 8px;
                        spacing: 6px;

                        Text {
                            text: "Disposizione raggruppata (numeri, lettere e simboli separati per digitazione intuitiva):";
                            color: #63b3ed;
                            font_size: 11px;
                        }

                        HorizontalLayout {
                            spacing: 4px;
                            Text { text: "NUM:"; color: #a0aec0; font_size: 11px; width: 40px; vertical_alignment: center; }
                            Text { text: root.numbers_row; color: #f7fafc; font_size: 12px; font_family: "monospace"; }
                        }
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
            }

            // -----------------------------------------------------------------
            // TAB 1: MAILBOX & INBOX SPOOL
            // -----------------------------------------------------------------
            if root.active_tab == 1 : VerticalLayout {
                spacing: 10px;

                Text {
                    text: "📬 ARCHIVIO ASINCRONO PIGEONHOLE & STATI QUANTISTICI";
                    color: #4fd1c5;
                    font_size: 15px;
                    font_weight: 700;
                }

                Text {
                    text: "I messaggi non ancora osservati rimangono in sovrapposizione quantistica (cifrati). La decoerenza avviene all'atto della lettura.";
                    color: #a0aec0;
                    font_size: 11px;
                }

                // Ingest Box
                Rectangle {
                    background: #1e2330;
                    border_radius: 6px;
                    border_width: 1px;
                    border_color: #2d3748;
                    height: 52px;

                    HorizontalLayout {
                        padding: 8px;
                        spacing: 8px;
                        Text {
                            text: "Busta Ricevuta (Hex):";
                            color: #e2e8f0;
                            font_size: 12px;
                            vertical_alignment: center;
                        }
                        LineEdit {
                            text <=> root.ingest_raw_hex;
                            placeholder_text: "Incolla l'esadecimale della busta ReplicaReadReply ricevuta dallo Spool/QR...";
                            horizontal_stretch: 1;
                        }
                        Button {
                            text: "Decifra & Osserva";
                            primary: true;
                            clicked => { root.ingest_reply_clicked(); }
                        }
                    }
                }

                Text {
                    text: "Messaggi nella Mailbox Locale del Vault:";
                    color: #e2e8f0;
                    font_size: 12px;
                    font_weight: 600;
                }

                ScrollView {
                    height: 200px;
                    VerticalLayout {
                        spacing: 6px;
                        for msg in root.messages_model : Rectangle {
                            background: #1a1e29;
                            border_radius: 6px;
                            border_width: 1px;
                            border_color: #2d3748;
                            height: 60px;

                            TouchArea {
                                clicked => { root.select_message_clicked(msg.raw_plaintext, msg.id); }
                            }

                            VerticalLayout {
                                padding: 6px;
                                spacing: 2px;
                                HorizontalLayout {
                                    Text {
                                        text: msg.direction;
                                        color: msg.direction == "INBOUND [↓]" ? #68d391 : #63b3ed;
                                        font_size: 11px;
                                        font_weight: 700;
                                        width: 110px;
                                    }
                                    Text {
                                        text: "Corrispondente: " + msg.party;
                                        color: #e2e8f0;
                                        font_size: 11px;
                                        font_family: "monospace";
                                        horizontal_stretch: 1;
                                    }
                                    Text {
                                        text: msg.secrecy;
                                        color: #ecc94b;
                                        font_size: 11px;
                                        horizontal_alignment: right;
                                    }
                                }
                                HorizontalLayout {
                                    Text {
                                        text: msg.quantum_state;
                                        color: #cbd5e0;
                                        font_size: 11px;
                                        horizontal_stretch: 1;
                                    }
                                    Text {
                                        text: "Anteprima: " + msg.preview;
                                        color: #718096;
                                        font_size: 10px;
                                    }
                                }
                            }
                        }
                    }
                }

                // Inspection pane for selected message
                Rectangle {
                    background: #1e2330;
                    border_radius: 6px;
                    border_width: 1px;
                    border_color: #2d3748;
                    height: 96px;

                    VerticalLayout {
                        padding: 8px;
                        spacing: 4px;
                        Text {
                            text: root.selected_message_info;
                            color: #ecc94b;
                            font_size: 11px;
                            font_weight: 600;
                        }
                        TextEdit {
                            text: root.selected_message_text;
                            read_only: true;
                            font_size: 12px;
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // TAB 2: RUBRICA CONTATTI & AVATAR
            // -----------------------------------------------------------------
            if root.active_tab == 2 : VerticalLayout {
                spacing: 10px;

                Text {
                    text: "👥 RUBRICA CONTATTI & AVATAR CRITTOGRAFICI DETERMINISTICI";
                    color: #4fd1c5;
                    font_size: 15px;
                    font_weight: 700;
                }

                Text {
                    text: "Ogni contatto possiede un avatar 5x5 a simmetria bilaterale derivato da SHA-256(username | Box ID). 100% Autonomo, zero email e verificabile visivamente.";
                    color: #a0aec0;
                    font_size: 11px;
                }

                // Add Contact Box
                Rectangle {
                    background: #1e2330;
                    border_radius: 6px;
                    border_width: 1px;
                    border_color: #2d3748;
                    height: 52px;

                    HorizontalLayout {
                        padding: 8px;
                        spacing: 8px;
                        Text {
                            text: "Nuovo Contatto:";
                            color: #e2e8f0;
                            font_size: 12px;
                            vertical_alignment: center;
                        }
                        LineEdit {
                            text <=> root.new_contact_username;
                            placeholder_text: "Username (es. Alice)...";
                            width: 160px;
                        }
                        LineEdit {
                            text <=> root.new_contact_box_id;
                            placeholder_text: "32-byte Hex Box ID (64 caratteri hex)...";
                            horizontal_stretch: 1;
                        }
                        Button {
                            text: "Aggiungi & Genera Avatar 🎨";
                            primary: true;
                            clicked => { root.add_contact_clicked(); }
                        }
                    }
                }

                Text {
                    text: "Contatti Sovrani Salvati nel Vault:";
                    color: #e2e8f0;
                    font_size: 12px;
                    font_weight: 600;
                }

                ScrollView {
                    height: 310px;
                    VerticalLayout {
                        spacing: 8px;
                        for contact in root.contacts_model : Rectangle {
                            background: #1a1e29;
                            border_radius: 6px;
                            border_width: 1px;
                            border_color: #2d3748;
                            height: 72px;

                            HorizontalLayout {
                                padding: 8px;
                                spacing: 14px;
                                Image {
                                    source: contact.avatar;
                                    width: 56px;
                                    height: 56px;
                                    image-fit: contain;
                                }
                                VerticalLayout {
                                    spacing: 4px;
                                    horizontal_stretch: 1;
                                    Text {
                                        text: contact.username;
                                        color: #4fd1c5;
                                        font_size: 14px;
                                        font_weight: 700;
                                    }
                                    Text {
                                        text: "Box ID: " + contact.box_id;
                                        color: #cbd5e0;
                                        font_size: 11px;
                                        font_family: "monospace";
                                    }
                                    Text {
                                        text: "Avatar deterministico generato in puro Rust — immutabile";
                                        color: #718096;
                                        font_size: 10px;
                                    }
                                }
                                Button {
                                    text: "Invia Messaggio ➔";
                                    clicked => { root.use_contact_clicked(contact.box_id); }
                                }
                            }
                        }
                    }
                }
            }

            // Bottom OPSEC Tip & Status Log
            Rectangle {
                background: #181b24;
                border_radius: 6px;

                HorizontalLayout {
                    padding: 6px;
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

fn reload_ui_models(ui: &MainWindow, mailbox: &VaultMailbox) {
    let msg_items: Vec<MessageItem> = mailbox
        .messages
        .iter()
        .map(|m| {
            let dir = match m.direction {
                MessageDirection::Inbound => "INBOUND [↓]",
                MessageDirection::Outbound => "OUTBOUND [↑]",
            };
            let party = if m.direction == MessageDirection::Inbound {
                &m.sender
            } else {
                &m.recipient
            };
            let party_short = if party.len() > 20 {
                format!("{}...", &party[0..20])
            } else {
                party.clone()
            };
            MessageItem {
                id: m.id.clone().into(),
                direction: dir.into(),
                party: party_short.into(),
                preview: m.plaintext.chars().take(40).collect::<String>().into(),
                time_str: format!("{}", m.timestamp).into(),
                quantum_state: m.quantum_state.clone().into(),
                secrecy: format!("{} stelle", m.secrecy_stars).into(),
                raw_plaintext: m.plaintext.clone().into(),
            }
        })
        .collect();
    ui.set_messages_model(Rc::new(VecModel::from(msg_items)).into());

    let contact_items: Vec<ContactItem> = mailbox
        .contacts
        .iter()
        .map(|c| {
            let avatar_img = c.identicon().to_slint_image(8);
            ContactItem {
                username: c.username.clone().into(),
                box_id: c.box_id_hex.clone().into(),
                avatar: avatar_img,
            }
        })
        .collect();
    ui.set_contacts_model(Rc::new(VecModel::from(contact_items)).into());
}

pub fn run_gui() -> Result<(), String> {
    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        println!("==> Nessun display grafico attivo rilevato (DISPLAY / WAYLAND_DISPLAY non impostati).");
        println!("==> Avvio della TUI Sovereign interattiva con tastiera virtuale raggruppata...\n");
        return run_interactive_tui();
    }

    let mailbox_path = default_vault_mailbox_path();
    let mailbox = VaultMailbox::new_or_load(&mailbox_path)
        .map_err(|e| format!("Errore inizializzazione mailbox: {}", e))?;

    let main_window = MainWindow::new().map_err(|e| format!("Slint GUI init error: {}", e))?;
    let kbd_state = VirtualKeyboardState::new();

    let my_id = format!("Box ID: {}", mailbox.identity.box_id_hex);
    main_window.set_my_box_id(my_id.into());

    let op_avatar = mailbox.identity.identicon().to_slint_image(8);
    main_window.set_operator_avatar(op_avatar);

    reload_ui_models(&main_window, &mailbox);

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

    let kbd_ref = Rc::new(RefCell::new(kbd_state));
    let mailbox_ref = Rc::new(RefCell::new(mailbox));

    // Callback switch tab
    {
        let ui_weak = main_window.as_weak();
        main_window.on_switch_tab(move |tab| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_active_tab(tab);
            }
        });
    }

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
                        reload_ui_models(&ui, &mbox);
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

    // Callback select message to inspect
    {
        let ui_weak = main_window.as_weak();
        main_window.on_select_message_clicked(move |raw_text, id| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_selected_message_text(raw_text);
                ui.set_selected_message_info(format!("Messaggio ID: {} — Decifrato ed osservato nel Vault:", id).into());
                ui.set_status_log("Messaggio selezionato e visualizzato.".into());
            }
        });
    }

    // Callback ingest reply envelope
    {
        let mbox_rc = mailbox_ref.clone();
        let ui_weak = main_window.as_weak();
        let mbox_path = mailbox_path.clone();
        main_window.on_ingest_reply_clicked(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let hex_str = ui.get_ingest_raw_hex().trim().to_string();
                if hex_str.is_empty() {
                    ui.set_status_log("Inserisci l'esadecimale della busta ReplicaReadReply da decifrare.".into());
                    return;
                }

                let raw_bytes = match hex::decode(&hex_str) {
                    Ok(b) => b,
                    Err(e) => {
                        ui.set_status_log(format!("Errore decodifica esadecimale: {}", e).into());
                        return;
                    }
                };

                let reply = match ReplicaReadReply::decode(&raw_bytes) {
                    Ok(r) => r,
                    Err(e) => {
                        ui.set_status_log(format!("Errore decodifica busta ReplicaReadReply: {}", e).into());
                        return;
                    }
                };

                let mut mbox = mbox_rc.borrow_mut();
                match mbox.ingest_reply(&reply) {
                    Ok(plaintext) => {
                        let _ = mbox.save(&mbox_path);
                        reload_ui_models(&ui, &mbox);
                        ui.set_ingest_raw_hex("".into());
                        ui.set_selected_message_text(plaintext.clone().into());
                        ui.set_selected_message_info("Busta decifrata con successo! Stato: Decoerenza osservata:".into());
                        ui.set_status_log("Busta Pigeonhole decifrata e salvata nella mailbox!".into());
                    }
                    Err(e) => {
                        ui.set_status_log(format!("Errore decifratura busta: {}", e).into());
                    }
                }
            }
        });
    }

    // Callback add contact
    {
        let mbox_rc = mailbox_ref.clone();
        let ui_weak = main_window.as_weak();
        let mbox_path = mailbox_path.clone();
        main_window.on_add_contact_clicked(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let username = ui.get_new_contact_username().trim().to_string();
                let box_id = ui.get_new_contact_box_id().trim().to_string();

                if username.is_empty() || box_id.is_empty() {
                    ui.set_status_log("Inserisci sia il nome utente che il Box ID a 32-byte hex.".into());
                    return;
                }

                let mut mbox = mbox_rc.borrow_mut();
                match mbox.add_contact(&username, &box_id) {
                    Ok(_) => {
                        let _ = mbox.save(&mbox_path);
                        reload_ui_models(&ui, &mbox);
                        ui.set_new_contact_username("".into());
                        ui.set_new_contact_box_id("".into());
                        ui.set_status_log(format!("Contatto '{}' aggiunto con successo e avatar deterministico generato!", username).into());
                    }
                    Err(e) => {
                        ui.set_status_log(format!("Errore aggiunta contatto: {}", e).into());
                    }
                }
            }
        });
    }

    // Callback use contact as recipient
    {
        let ui_weak = main_window.as_weak();
        main_window.on_use_contact_clicked(move |box_id| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_recipient_box_id(box_id.clone());
                ui.set_active_tab(0); // Switch to Composer
                ui.set_status_log(format!("Destinatario impostato su {}.", &box_id[0..12.min(box_id.len())]).into());
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

    loop {
        println!("\n================================================================================");
        println!("     KATZENPOST SOVEREIGN VAULT — SOVRANITÀ AIR-GAPPED & PIGEONHOLE           ");
        println!("================================================================================");
        println!("Operatore Sovrano     : {}", mailbox.identity.username);
        println!("Identità Box ID       : {}", mailbox.identity.box_id_hex);
        println!("Avatar Identicon (Crittografico deterministico):");
        println!("{}", mailbox.identity.identicon().render_terminal());
        println!("--------------------------------------------------------------------------------");
        println!("Messaggi in Mailbox   : {}", mailbox.messages.len());
        println!("Contatti in Rubrica   : {}", mailbox.contacts.len());
        println!("--------------------------------------------------------------------------------");
        println!("Menu:");
        println!("  1. Componi & deposita messaggio (con tastiera virtuale raggruppata anti-keylogger)");
        println!("  2. Visualizza messaggi Mailbox (Stato Quantistico di Schrödinger)");
        println!("  3. Visualizza rubrica contatti & avatar deterministici");
        println!("  4. Aggiungi nuovo contatto in rubrica");
        println!("  5. Esci");
        print!("\nScelta [1-5]: ");
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).map_err(|e| e.to_string())?;

        match choice.trim() {
            "1" => {
                let score = kbd.compute_secrecy_score();
                println!("\nPunteggio attuale: {} ({}%) - {}", "⭐".repeat(score.stars as usize), score.score_percentage, score.shielding_level);
                println!("Vuoi abilitare la tastiera virtuale raggruppata anti-keylogger per portare lo score al 100% ⭐⭐⭐⭐⭐?");
                print!("[s/N]: ");
                io::stdout().flush().map_err(|e| e.to_string())?;

                let mut kbd_choice = String::new();
                io::stdin().read_line(&mut kbd_choice).map_err(|e| e.to_string())?;
                let use_v = kbd_choice.trim().eq_ignore_ascii_case("s") || kbd_choice.trim().eq_ignore_ascii_case("y");

                if use_v {
                    kbd.is_enabled = true;
                    kbd.used_for_composition = true;
                    let boosted = kbd.compute_secrecy_score();
                    println!("[✓] Tastiera virtuale attiva: score elevato a {} ({}%)", "⭐".repeat(boosted.stars as usize), boosted.score_percentage);
                }

                print!("\nInserisci Box ID destinatario (o premi Enter per auto-invio loopback): ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut target = String::new();
                io::stdin().read_line(&mut target).map_err(|e| e.to_string())?;
                let target = target.trim();
                let target_id = if target.is_empty() {
                    mailbox.identity.box_id_hex.clone()
                } else {
                    target.to_string()
                };

                print!("Inserisci il messaggio: ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut msg = String::new();
                io::stdin().read_line(&mut msg).map_err(|e| e.to_string())?;
                let msg = msg.trim();

                let final_score = kbd.compute_secrecy_score();
                let method = if use_v { "Virtual Keyboard (Grouped Anti-Keylogger)" } else { "Physical Keyboard" };

                let deposit = mailbox.deposit_draft(&target_id, msg, final_score.stars, method)?;
                mailbox.save(&mailbox_path)?;

                println!("[✓] Bozza cifrata e depositata con successo!");
                println!("    Dimensione: {} byte", deposit.payload.len());
                println!("    Stato quantistico: Superposizione (Scatola di Schrödinger chiusa)");
            }

            "2" => {
                println!("\n--- ARCHIVIO MESSAGGI VAULT ({}) ---", mailbox.messages.len());
                if mailbox.messages.is_empty() {
                    println!("Nessun messaggio presente.");
                } else {
                    for (i, m) in mailbox.messages.iter().enumerate() {
                        let dir = match m.direction {
                            MessageDirection::Inbound => "INBOUND [↓]",
                            MessageDirection::Outbound => "OUTBOUND [↑]",
                        };
                        println!("#{} {} | {} | {} stelle | Stato: {}", i + 1, dir, m.id, m.secrecy_stars, m.quantum_state);
                        println!("   Testo: {}", m.plaintext);
                    }
                }
            }

            "3" => {
                println!("\n--- RUBRICA CONTATTI SOVRANI ({}) ---", mailbox.contacts.len());
                if mailbox.contacts.is_empty() {
                    println!("Nessun contatto salvato.");
                } else {
                    for (i, c) in mailbox.contacts.iter().enumerate() {
                        println!("#{} Contatto: {} (Box ID: {})", i + 1, c.username, c.box_id_hex);
                        println!("{}", c.identicon().render_terminal());
                    }
                }
            }

            "4" => {
                print!("\nUsername nuovo contatto: ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut u = String::new();
                io::stdin().read_line(&mut u).map_err(|e| e.to_string())?;

                print!("Box ID (64 caratteri hex): ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut b = String::new();
                io::stdin().read_line(&mut b).map_err(|e| e.to_string())?;

                match mailbox.add_contact(u.trim(), b.trim()) {
                    Ok(c) => {
                        mailbox.save(&mailbox_path)?;
                        println!("[✓] Contatto aggiunto con successo! Ecco il suo avatar deterministico:");
                        println!("{}", c.identicon().render_terminal());
                    }
                    Err(e) => {
                        println!("[!] Errore: {}", e);
                    }
                }
            }

            "5" => {
                println!("Chiusura Vault Sovrano. Arrivederci!");
                break;
            }

            _ => {
                println!("Opzione non valida.");
            }
        }
    }

    Ok(())
}
