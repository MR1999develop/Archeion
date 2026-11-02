# Stato del progetto

Aggiornato alla fine di ogni blocco di lavoro (vedi `CLAUDE.md`).

## Ultimo commit controllato

`main` dopo il passaggio a GPLv3 (`tools\check.bat` verde: fmt, clippy,
294 test; `tools\release.bat` verde, nessun percorso locale nell'exe).

## Lavoro in corso

- Nessuno.

## Punti aperti

- Il nome completo e l'email restano nei due commit precedenti a GPLv3
  (autore dei commit e vecchi file di licenza): da riscrivere prima del
  primo push, se si vuole una cronologia pulita.
- `Archeion`, il primo nome del programma, resta nel codice di migrazione
  (cartella dati, Credential Manager, task pianificati) e nei documenti che
  lo spiegano: serve a ritrovare i dati delle installazioni precedenti.

## Decisioni

- Licenza `GPL-3.0-only`, copyright e autori `MR`.
- `tools\release.ps1` passa `--remap-path-prefix` e `/PDBALTPATH` con
  `CARGO_ENCODED_RUSTFLAGS` invece che da `.cargo/config.toml`: i percorsi
  da riscrivere sono locali e non devono finire in un file versionato.
  Lo script poi cerca la cartella del progetto, la cartella utente e
  quella di cargo nell'exe e fallisce se le trova.
- Gli script git non impostano più un autore predefinito: senza
  `user.name`/`user.email` si fermano.
