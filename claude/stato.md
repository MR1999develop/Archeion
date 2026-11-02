# Stato del progetto

Aggiornato alla fine di ogni blocco di lavoro (vedi `CLAUDE.md`).

## Ultimo commit controllato

`main` con i test end-to-end e della finestra (ramo
`feature/e2e-tests` unito; `tools\check.bat` verde: fmt, clippy,
310 test). La build di rilascio è stata controllata al commit GPLv3
(nessun percorso locale nell'exe).

## Lavoro in corso

- Nessuno. Da guardare a mano solo l'aspetto dell'albero di ripristino
  (vedi `TIDY.md`).

## Punti aperti

- Nessuno.

## Decisioni

- Licenza `GPL-3.0-only`, copyright e autori `MR`. Cronologia riscritta
  (2 novembre) senza nome, email né percorsi locali; autore dei commit
  `MR <MR1999develop@users.noreply.github.com>`, impostato anche nella
  configurazione git del repository.
- `tools\release.ps1` passa `--remap-path-prefix` e `/PDBALTPATH` con
  `CARGO_ENCODED_RUSTFLAGS` invece che da `.cargo/config.toml`: i percorsi
  da riscrivere sono locali e non devono finire in un file versionato.
  Lo script poi cerca la cartella del progetto, la cartella utente e
  quella di cargo nell'exe e fallisce se le trova.
- Il primo nome del programma è tolto ovunque, cronologia compresa: niente
  migrazione da installazioni precedenti; chiavi derivate con contesti
  "Archeion 2026-11-02 … v3" e `KEY_FORMAT` 3, quindi le destinazioni
  cifrate create prima non si aprono più (messaggio "test version no
  longer supported") e vanno ricreate.
- Gli script git non impostano più un autore predefinito: senza
  `user.name`/`user.email` si fermano.
- Albero di ripristino: una cartella spuntata spunta tutto ciò che
  conteneva alla sua data e sostituisce le scelte fatte dentro di essa; ciò
  che si toglie dentro diventa un'esclusione (`RestoreItem::exclude`, letta
  dal motore con `RestoreItem::wants`). Rispuntare un elemento dentro una
  sottocartella esclusa sostituisce quell'esclusione con le sorelle lungo
  il percorso (`Tree::include`). Lo stato si calcola in `Tree::tick`,
  testato senza egui.
  Lo stato parziale è la casella "indeterminate" di egui riempita
  (`fill_box`). Con più schede, solo quella evidenziata mostra le opzioni.
- Prove autonome: `app/tests/cli.rs` esegue `archeion.exe` su cartelle
  temporanee (`ARCHEION_DATA_DIR`, `ARCHEION_PASSWORD`), i test egui_kittest
  guidano la vista di ripristino con i font di sistema come eframe
  (`egui_system_font_provider`), un test controlla che ogni carattere
  dell'interfaccia e delle 9 lingue si possa disegnare. Permessi dei
  comandi ripetitivi in `.claude/settings.json` (push e publish negati).
