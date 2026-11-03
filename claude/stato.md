# Stato del progetto

Aggiornato alla fine di ogni blocco di lavoro (vedi `CLAUDE.md`).

## Ultimo commit controllato

`main` con la scelta della versione dal nome del file (ramo
`feature/restore-versions` unito; `tools\check.bat` verde: fmt, clippy, 334
test, 1 ignorato: le schermate). La build di rilascio è stata controllata al commit GPLv3
(nessun percorso locale nell'exe).

## Lavoro in corso

- Nessuno.

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
- Aspetto: tutti i colori, le misure e i font in `ui/theme.rs` (token
  chiaro/scuro in stile Windows 11, contrasti testati), i componenti
  comuni in `ui/widgets.rs` (titoli di sezione, schede, bottone primario e
  distruttivo, finestre e modali), il font di sistema in `ui/platform.rs`
  (Segoe UI letto da Windows, mai distribuito). Le schermate di tutte le
  viste si generano con il test ignorato `screens` (egui_kittest con wgpu)
  e passano il controllo di accessibilità di kittest.
- Icone dal font di sistema Segoe MDL2 Assets (letto da `%WINDIR%\Fonts`,
  mai incluso), costanti in `ui/symbol.rs` con i codici verificati
  disegnandoli; i testi tradotti che nominano un'icona la ricevono come
  variabile (`{ $deleted-icon }`, `{ $edited-icon }`, `{ $remove-icon }`).
  Caselle spuntate riempite con l'accento, liste con l'elemento selezionato
  intero e il segno d'accento, pulsanti-icona senza bordo
  (`widgets::checkbox`, `partial_checkbox`, `list_item`, `icon_button`).
- Dimensione della finestra in `Settings::window_size` (pixel logici,
  `u16`, più "ingrandita"): salvata dopo 1 s senza ridimensionamenti o
  alla chiusura, applicata all'avvio mai sotto il minimo 760×480. La
  posizione non si salva: Windows la sceglie (niente finestre fuori da
  uno schermo staccato).
- Prima dimensione 1200×1000 (tutte le impostazioni di un profilo senza
  scorrere), ingrandita se lo schermo è più basso di 1100 px logici
  (Full HD compreso);
  controlli alti 24 e righe a 6 px (`theme::CONTROL_HEIGHT`, `ROW_GAP`).
  Le schermate di prova usano la stessa prima dimensione.
- Toolchain fissata a 1.99.1 (`rust-toolchain.toml`, CI, MSRV 1.99): il
  canale "1.99" installato in locale ha la libreria standard incompleta.
  `fs4` al posto di `fs2` (abbandonato) per lo spazio libero; un test con
  un valore noto (argon2-cffi) garantisce che le chiavi derivate dalle
  password non cambino aggiornando `argon2`.
- Impostazioni del profilo larghe `theme::SETTINGS_MAX_WIDTH` (760; la
  Guida e gli altri moduli restano a `FORM_MAX_WIDTH` 600 per la lettura),
  il nome come riga e non come sezione, spazio libero accanto alla
  destinazione: con la finestra 1200×1000 finiscono a circa 655 px.
- Profili come file `.archeion-profile` (`app/src/profile_file.rs`): TOML
  con `format = 1` e il profilo intero, mai la password; al massimo 1 MiB;
  un formato successivo è rifiutato. L'importazione mantiene l'id (su un
  altro PC il profilo ritrova i suoi backup) salvo che esista già: allora
  id nuovo e nome con "(2)".
- L'eseguibile è sempre un programma a finestre (`windows_subsystem`,
  tranne i test unitari): niente console accanto alla finestra; i comandi
  scrivono su stdout letto tramite pipe. Comandi a mano nel terminale
  senza output: in `TIDY.md`.
- Albero di ripristino: una cartella non spuntata il cui contenuto è
  tutto spuntato elemento per elemento è `Tick::Full` (casella spuntata);
  un clic toglie tutte le spunte all'interno invece di trasformarle nella
  cartella spuntata (le versioni scelte dei file non cambiano finché
  l'utente non lo chiede).
- Ripristino: una sola destinazione per tutta la selezione (le 3 opzioni
  in alto, più "Sostituisci i file già presenti"); tolti il menu della
  destinazione di ogni versione o cartella e l'icona ✎. Restano le
  versioni di un file e la data di una cartella. "Togli selezione" è un
  pulsante accanto a Ripristina.
- Ripristino senza schede (richiesta dell'utente, 3 novembre): un file
  torna nella versione più recente, una cartella com'era nel backup più
  recente che la contiene. Una versione o una data più vecchia non si
  sceglie più dalla finestra.
- Backup del profilo eliminato nel Cestino (`archeion_engine::trash_destination`,
  crate `trash`): solo se il descrittore è di quel profilo; la cartella
  intera se contiene solo ciò che ha scritto Archeion, altrimenti ogni
  nome di Archeion (descrittore per ultimo). Windows cancella senza dirlo
  ciò che non può cestinare (condivisioni di rete, dischi rimovibili): un
  file di prova va prima nel Cestino e dev'esserci, e ogni elemento
  spostato si cerca nel Cestino; al primo che manca ci si ferma. Niente
  Annulla per il profilo; la password salvata si toglie subito. I test
  svuotano dal Cestino ciò che vi mettono.
- Unione: `MergePolicy::keep_deleted` (vero per default, anche per i
  profili salvati prima) fa entrare nel manifesto unito le voci elencate
  da un membro del gruppo ma non dall'ultimo, com'erano nell'ultimo che
  le elencava. Vale per l'unione automatica e per quella a mano (casella
  propria, che parte dal valore del profilo; `--drop-deleted` la spegne
  per quell'unione). L'eliminazione di un singolo backup scarta sempre
  ciò che conteneva solo lui.
- Albero di ripristino: la casella è senza testo (nome accessibile per i
  lettori di schermo) e sola a spuntare; il nome di un file è un pulsante
  che mostra a destra le sue versioni (`RestoreView::shown`), la scelta
  è in `RestoreView::chosen` e vale anche se fatta prima di spuntare. Nei
  test, un clic su un pulsante appena attivato va fatto un fotogramma
  dopo (egui verifica i clic sul fotogramma precedente).
