# Stato del progetto

Aggiornato alla fine di ogni blocco di lavoro (vedi `CLAUDE.md`).

## Ultimo commit controllato

`main` con le spunte dell'albero di ripristino (ramo
`feature/restore-tree-ticks` unito; `tools\check.bat` verde: fmt, clippy,
297 test). La build di rilascio è stata controllata al commit GPLv3
(nessun percorso locale nell'exe).

## Lavoro in corso

- Nessuno. Da provare a mano: albero di ripristino (vedi `TIDY.md`).

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
- Albero di ripristino: una cartella spuntata decide per tutto ciò che
  contiene (spunte grigie, non modificabili) e sostituisce le scelte fatte
  dentro di essa; lo stato si calcola in `Tree::tick`, testato senza egui.
  Lo stato parziale è la casella "indeterminate" di egui riempita
  (`fill_box`). Con più schede, solo quella evidenziata mostra le opzioni.
