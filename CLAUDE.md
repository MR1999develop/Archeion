# Istruzioni di sviluppo di Archeion

- Lo sviluppo del progetto avviene in una sola chat, che scrive il codice,
  compila, controlla, corregge e unisce i rami.
- Si sviluppa in locale con git e GitHub; il push su GitHub lo fa l'utente a
  mano. Niente push o release senza una sua richiesta esplicita.
- Un ramo di lavoro alla volta, partendo dall'ultimo `main` controllato; si
  unisce a `main` solo dopo `tools\check.bat` tutto verde.
- Rispondere in italiano; interfaccia, codice, commenti e commit in inglese.
- Applicare le skill di progettazione prima di scrivere o rivedere codice:
  `rust-egui-desktop-design`, `software-design-core`, `ui-design-core` e,
  per crittografia, password e rilascio, `security-design`.
- Linguaggio: Rust + egui.
- Alla fine di ogni blocco di lavoro aggiornare `TIDY.md` (cose rimandate) e
  il documento di stato del progetto `claude/stato.md` (ultimo commit
  controllato, lavoro in corso, punti aperti, decisioni).
- Licenza `GPL-3.0-only` (testo in `LICENSE`), copyright e autori **MR**;
  nessun dato personale (nome, email, percorsi locali) nei file né negli
  eseguibili: `tools\release.bat` riscrive i percorsi e controlla l'exe.
