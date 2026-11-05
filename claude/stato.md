# Stato del progetto

Aggiornato alla fine di ogni blocco di lavoro (vedi `CLAUDE.md`).

## Ultimo commit controllato

`main` con i rami della revisione di sicurezza (`fix/security-review-2`:
cartelle di rete da confermare, ripristino limitato alla dimensione,
regole delle password), della catena di fornitura (`chore/supply-chain-2`:
controllo giornaliero, SBOM, firma facoltativa), dei test di proprietà
(`test/property-parsers`: proptest sui lettori di manifest, chiavi, righe
del worker, esclusioni tipizzate, esadecimale e note sigillate, file di
profilo) e della terza revisione di sicurezza (`fix/security-review-3`,
4 novembre: giornale delle unioni solo con nomi di archivio, firma
dell'elenco controllata a ogni apertura, percorsi di rete in ogni grafia,
collegamenti e junction non seguiti, file piccoli limitati a 1 MiB,
password lasciata da un profilo eliminato, confronti a tempo costante,
release in due job; test in `crates/engine/tests/hostile.rs` e in
`app/tests/cli.rs`) e del recupero delle unioni (`fix/merge-recovery-copy`:
un archivio unito illeggibile si mette da parte come `.damaged` invece di
far cancellare il membro più recente, caso segnalato dalla scheda OSTEP)
e della correzione di `tools\github_publish.ps1` (`fix/publish-script-parse`:
errore di sintassi alla riga 69, `$Repo:`; nuovo passo 6/6 di
`tools\check.bat` e della CI, `tools\script_check.ps1`, che analizza
tutti gli script PowerShell senza eseguirli) e del nome esatto del
repository (`chore/repository-name`: `MR1999develop/Archeion` con la
maiuscola, come su GitHub, in `origin`, `Cargo.toml`, README, SECURITY,
CHANGELOG, finestra Aiuto e script) e della copia del profilo nella
destinazione (`feature/profile-copy`: ogni backup scrive
`archeion.profile.toml`, o `archeion.profile.zip` sigillato se cifrato;
File > Importa profilo da un backup… la legge; formato del testo spostato
in `archeion_core::profile_text`; test in
`crates/engine/tests/profile_copy.rs`, `app/src/ui/backup_import.rs` e
`app/tests/cli.rs`), del test completo dell'importazione
(`test/full-import`) e dell'elenco dei profili (`feature/profile-list`, 5
novembre: ordine per trascinamento e con Alt+Su/Giù, menu del tasto
destro e del menu File con Rinomina sul posto e Elimina profilo…, niente
più pulsante sotto le impostazioni; modulo `ui/profile_list.rs`, test in
`app/src/ui/app_tests.rs`) e della conferma delle regole di unione
(`feature/merge-preview`: `MergePolicy::approved`, `merge_plan::preview`,
`archive_names`; test in `merge_plan.rs`, `crates/engine/tests/merge_wait.rs`
e `app_tests.rs`); `tools\check.bat` verde.
Su GitHub (4 novembre) c'è `main` fino a `542a215`; il resto parte con il
prossimo push, che lancia l'utente.

## Lavoro in corso

- Nessuno.

## Punti aperti

- Provare a mano: Annulla durante un backup (si ferma al file successivo),
  spegnimento con un backup in corso (messaggio di Windows), seconda
  apertura che porta avanti la finestra, profilo importato con la
  pianificazione in pausa, avviso sulle unità FAT32/exFAT.
- Misurare su un disco USB lento il costo del flush per file (TIDY).
- GitHub: vedere verde la prima esecuzione della CI, poi le impostazioni
  del repository (regole per `main` e per i tag `v*`, sicurezza del
  codice) e, per ultimo, il cambio di visibilità (TIDY).
- Il workflow di rilascio (due job, passaggio dei file come artefatto,
  controllo che il tag sia su `main`, attestazioni) non è ancora stato
  eseguito su GitHub.
- Misurare su un NAS il costo del controllo del percorso reale dei file
  letti dai backup in chiaro (TIDY).

## Decisioni

- Destinazione non fidata anche per dove si legge e si scrive sul PC (4
  novembre): il giornale delle unioni contiene solo `ArchiveName` (serde
  in `archeion_core::archive`), la ripresa tocca solo cartelle
  `<archivio>.old/.merging/.deleting`, le chiavi si aprono prima della
  ripresa. Collegamenti e junction: un backup interrotto che ne contiene
  non si riprende; i file in chiaro si leggono solo se il loro percorso
  reale (`archeion_win::real_path`) è dentro la destinazione. La firma
  dell'elenco si controlla a ogni apertura: backup, unione ed eliminazione
  avvisano (`changed_outside` nei rapporti, notifica e nota nel
  riepilogo), poi firmano l'elenco trovato; la ripresa non rifirma un
  elenco che non corrisponde.
- GitHub con un token fine-grained solo per Archeion (4 novembre), come
  l'altro progetto: Git Credential Manager lo tiene per l'URL del
  repository (`useHttpPath` in `.git/config`), `gh` resta con il token
  dell'altro progetto e non si usa per Archeion; con `origin` impostato
  `tools\github_publish.bat` non chiama `gh`. Il repository si chiama
  `Archeion` con la maiuscola: ogni riferimento usa quel nome esatto.
- Impostazioni dei profili sul PC, fonte di verità (4 novembre); ogni
  backup ne lascia una copia nella destinazione (scritta all'apertura,
  dopo il recupero: temporaneo, flush, rinomina), sigillata con la chiave
  se la destinazione è cifrata, e toglie quella dell'altro tipo. La copia
  si legge solo con File > Importa profilo da un backup…, su una cartella
  scelta dall'utente: l'id è quello del proprietario in `archeion.json`,
  la destinazione è la cartella scelta, la cifratura quella della
  destinazione; poi le stesse protezioni dei file di profilo
  (`profile_file::add`). La password che apre i backup si salva con il
  profilo. Scartate: impostazioni solo nella destinazione (non fidata,
  irraggiungibile quando il disco è staccato, memoria anti-rollback da
  tenere sul PC) e copia per archivio (più complessa con unioni ed
  eliminazioni, senza un bisogno attuale).
- Regole di unione (5 novembre): le regole nuove che unirebbero backup
  tenuti da quelle confermate restano in attesa (`merge.approved` tiene
  le confermate) e l'unione automatica non parte finché l'utente non
  sceglie "Unisci con le nuove regole" o "Ripristina le regole
  precedenti"; "Unisci ora" vale come conferma. Regole più larghe, o più
  strette ma senza effetto sui backup presenti, si confermano da sole; con
  i backup non ancora letti una modifica resta in attesa.
- La finestra riceve l'Utilità di pianificazione come dipendenza
  (`Tasks`, trait `archeion_scheduler::Scheduler` con `apply_watch`): i
  test della finestra usano un registratore e non toccano mai le attività
  dell'utente. Il test completo dell'importazione cifrata
  (`an_encrypted_profile_comes_back_from_its_backups_with_its_password`)
  scrive in Gestione credenziali vera con l'id casuale del profilo di
  prova e lo cancella alla fine (accettato dall'utente il 5 novembre).
- Il recupero non cancella mai l'unica copia (OSTEP): un `.old` si toglie
  solo se l'archivio al suo posto si legge; un archivio illeggibile va in
  `<archivio>.damaged`, che Archeion non cancella mai (il controllo lo
  nomina, l'eliminazione del profilo lo manda nel Cestino).
- Un percorso di un profilo importato è locale solo se è su un'unità di
  questo PC (`X:\…`, `\\?\X:\…`); ogni altra grafia chiede conferma
  (`archeion_core::profile::remote_part`).
- Licenza `GPL-3.0-only`, copyright e autori `MR`. Cronologia riscritta
  (2 novembre) senza nome, email né percorsi locali, e di nuovo il 4
  novembre prima della pubblicazione: gli esempi di rete e la cartella di
  prova sono generici (`\\server\…`, `E:\ArcheionTest`), nessun nome
  proprio nelle liste di prova. Esempi di percorsi e computer sempre
  inventati, mai quelli del PC su cui si sviluppa. Autore dei commit
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
- Versione scelta per un file dentro una cartella spuntata: il file
  entra nelle esclusioni della cartella e diventa una scelta a sé con
  quella versione; `Tree::tick` non conta come mancante un'esclusione
  che ha una scelta propria, così la cartella resta spuntata intera.
  Frecce su e giù: tra i file disegnati (albero o ricerca), solo quando
  nessun campo di testo ha la tastiera.
- Nome di un file nell'albero: `widgets::chosen_name` (etichetta cliccabile
  con lo sfondo e il segno d'accento della voce selezionata, cede il focus
  così le frecce restano all'albero). File assente dalla data di una
  cartella spuntata: spuntabile da solo; entra nelle esclusioni della
  cartella (niente conflitto quando si sostituisce l'originale) e ne esce
  quando lo si toglie.
- `RestoreItem::with_deleted` (la finestra lo mette sempre, la riga di
  comando no): una cartella aggiunge ciò che i backup precedenti alla sua
  data elencavano e lei no, ciascuno dal più recente che lo elenca; ogni
  file assente dal backup più recente prende il prefisso
  `restore::DELETED_PREFIX` ("_deleted_"). Nell'albero un file salvato
  prima della data della cartella risulta spuntato con lei. Il totale
  sopra Ripristina non conta ancora i file eliminati (in `TIDY.md`).
- Icona disegnata in codice (`ui/icon.rs`): finestra, area di notifica e,
  tramite `app/build.rs` (crate `winresource`, che usa `rc.exe` del
  Windows SDK), anche l'eseguibile, con nome, descrizione e copyright MR
  nelle Proprietà. Anteprima: `cargo test -p archeion --bin archeion
  icon_preview -- --ignored` scrive `target/ui-screens/archeion.ico`.
- Casella "Includi i file eliminati nelle cartelle spuntate" sopra
  l'albero (`RestoreView::skip_deleted`, attiva per default, tenuta come la
  destinazione): vale per tutte le cartelle spuntate, subito; arriva al
  motore come `with_deleted` delle cartelle (i file restano marcati).
- Tolti `tools\probe.bat` (esperimenti iniziali), `tools\git_init.bat` e
  `tools\first-commit.txt` (il repository esiste), le chiavi di traduzione
  delle funzioni rimosse. Resta `tools\github_publish.bat`: il repository
  non ha ancora un remote.
- Revisione con tutte le skill (3 novembre): la destinazione non è fidata.
  Con le chiavi si leggono solo manifest e file cifrati; un profilo
  cifrato o archivi con manifest cifrati non vanno mai avanti in chiaro
  (`EncryptionMissing`); le chiavi dei manifest sono controllate in
  lettura (`keys::check`).
- Il codice `unsafe` sta solo in `crates/win` (lint propri:
  `undocumented_unsafe_blocks`); il resto del workspace lo vieta.
- Durabilità: flush di ogni file prima del commit e rinomine write-through
  ai punti di commit; il recupero dei merge non cancella mai un `.old`
  se l'archivio unito non è al suo posto.
- Annulla chiude lo stdin del worker (`--stop-on-eof`): il motore si
  ferma al file successivo (`EventSink::stop_requested`); kill dopo 15 s.
  I worker della finestra stanno in un job object.
- Un profilo importato ha la pianificazione in pausa (`schedule_paused`)
  finché l'utente non la attiva.
- `destinations.json` nella cartella dati ricorda l'ultimo backup visto
  per profilo: se la destinazione torna indietro arriva una notifica (non
  un blocco). Anche un incrementale con molti più file cambiati del
  solito è notificato.
- `ROW_GAP` resta 6 (non multiplo di 4) per non far scorrere le
  impostazioni.
- Test profondi (`crates/engine/tests/deep.rs`, 3 novembre): storie
  casuali con seme stampato, interruzioni a ogni file, tutti gli stati di
  crash di un merge, fuzz dei manifest, dati danneggiati; proprietà delle
  chiavi in `keys.rs`. Le fughe del fuzz atterrano nella cartella
  temporanea: un primo giro con il controllo delle chiavi tolto per
  prova aveva creato cartelle vuote in `C:\` (tolte subito).

