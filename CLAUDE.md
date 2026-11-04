# Istruzioni di sviluppo di Archeion

- Lo sviluppo del progetto avviene in una sola chat, che scrive il codice,
  compila, controlla, corregge e unisce i rami.
- Si sviluppa in locale con git e GitHub; il push su GitHub lo fa l'utente a
  mano. Niente push o release senza una sua richiesta esplicita.
- Un ramo di lavoro alla volta, partendo dall'ultimo `main` controllato; si
  unisce a `main` solo dopo `tools\check.bat` tutto verde.
- Rispondere in italiano; interfaccia, codice, commenti e commit in inglese.
- Applicare le skill di progettazione prima di scrivere o rivedere codice:
  - sempre: `rust-egui-desktop-design`, `software-design-core` e
    `durable-data-design` (archivi, manifest, journal, merge, ripristino);
  - interfaccia: `ui-design-core`;
  - file e percorsi di Windows, processi e worker, Utilità di
    pianificazione, Credential Manager, area di notifica, copie shadow:
    `windows-system-design`;
  - crittografia, password, file non fidati e rilascio: `security-design`.
- Linguaggio: Rust + egui.
- Alla fine di ogni blocco di lavoro aggiornare `TIDY.md` (cose rimandate) e
  il documento di stato del progetto `claude/stato.md` (ultimo commit
  controllato, lavoro in corso, punti aperti, decisioni).
- Prove senza l'utente: `tools\check.bat` comprende i test end-to-end
  dell'eseguibile (`app/tests/cli.rs`: backup, ripristino con esclusioni,
  cifratura, verify, merge, delete, extract su cartelle temporanee, con
  `ARCHEION_DATA_DIR` e `ARCHEION_PASSWORD`, mai le impostazioni vere) e i
  test della finestra con egui_kittest (`app/src/ui/restore_tests.rs`).
  Ogni nuova funzione di backup o ripristino aggiunge un caso lì; un test
  nuovo si vede fallire iniettando un guasto prima di fidarsene.
- Interfaccia: colori, misure e font solo da `ui/theme.rs`, componenti da
  `ui/widgets.rs` (caselle, liste, pulsanti-icona: mai quelli di egui
  direttamente), icone solo da `ui/symbol.rs`; dopo ogni modifica visibile rigenerare e guardare le
  schermate (`cargo test -p archeion --bin archeion screens -- --ignored`,
  file in `target/ui-screens`), in italiano e inglese, chiaro e scuro.
- Licenza `GPL-3.0-only` (testo in `LICENSE`), copyright e autori **MR**;
  nessun dato personale (nome, email, percorsi locali) nei file né negli
  eseguibili: `tools\release.bat` riscrive i percorsi e controlla l'exe.
