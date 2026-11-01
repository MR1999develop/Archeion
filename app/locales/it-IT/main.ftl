# Menus
menu-file = File
menu-backup = Backup
menu-help = Aiuto
new-profile = Nuovo profilo
quit = Esci
restore = Ripristina file…
data-folder = Cartella dei dati

# Profile list and empty state
profiles = Profili
unnamed = (senza nome)
new-profile-name = Nuovo backup
empty-title = Ancora nessun backup
empty-body = Crea un profilo: scegli le cartelle da salvare, dove salvarle e quando.
tab-settings = Impostazioni
tab-backups = Backup

# Profile form
profile-name = Nome
folders = Cartelle da salvare
no-folders = Nessuna cartella: aggiungine almeno una.
add-folder = Aggiungi cartella…
remove-folder = Togli questa cartella dal backup
exclusions = Esclusioni
exclusions-hint = I modelli valgono sul percorso completo, per esempio **/node_modules o **/*.tmp.
destination = Destinazione
dest-local = Cartella o disco esterno
dest-network = Cartella di rete
dest-hint = Usa una cartella riservata a questo profilo. I backup non sono cifrati: chi può aprirla può leggerli.
browse = Sfoglia…
container = Formato
container-zip-per-file = Uno zip per file
container-folder = Copie semplici
container-zip-per-file-hint = Ogni file è compresso da solo (relazione.docx diventa relazione.docx.zip) nelle stesse cartelle; si apre con Esplora file o 7-Zip.
container-folder-hint = I file sono copiati così come sono, con gli stessi nomi, senza compressione.
schedule = Backup automatico
schedule-manual = Solo quando lo avvio io
schedule-daily = Ogni giorno
schedule-weekly = Ogni settimana
at = alle
schedule-hint = Se a quell'ora il computer è spento, parte appena si riaccende.
merge = Unione dei backup vecchi
merge-hint = I backup vecchi non vengono cancellati ma uniti al successivo che resta: ogni data conservata rimane completa e lo spazio si libera. Il primo backup completo non viene mai unito in automatico.
retention-hint = Un backup resta se almeno una regola lo sceglie.
keep-last = Più recenti
keep-daily = Uno al giorno, per giorni
keep-weekly = Uno a settimana, per settimane
keep-monthly = Uno al mese, per mesi
keep-yearly = Uno all'anno, per anni
keep-everything = Tutte le regole a 0: ogni backup resta e non si unisce nulla.
back-up-now = Esegui backup
merge-now = Unisci ora
merge-now-hint = Applica subito le regole di unione; il primo backup completo resta com'è.
busy-hint = Un backup o un'unione è già in corso.
fix-profile-hint = Completa prima il profilo.
delete-profile = Elimina profilo
delete-profile-hint = Toglie il profilo e le sue attività pianificate; i backup restano nella destinazione.

day-monday = lunedì
day-tuesday = martedì
day-wednesday = mercoledì
day-thursday = giovedì
day-friday = venerdì
day-saturday = sabato
day-sunday = domenica

error-empty-name = Dai un nome al profilo.
error-no-sources = Aggiungi almeno una cartella da salvare.
error-no-destination = Scegli dove salvare i backup.
error-destination-in-source = La destinazione è dentro una cartella da salvare; scegline un'altra.

# Backups tab
refresh = Aggiorna
reading-backups = Lettura dei backup…
no-backups-yet = Ancora nessun backup per questo profilo.
kind-complete = Completo
kind-incremental = Incrementale

# Status bar
ready = Pronto
cancel = Annulla
undo = Annulla eliminazione
phase-starting = Avvio…
phase-opening = Apertura della destinazione…
phase-scanning = Ricerca delle modifiche…
phase-copying = Copia in corso…
phase-merging = Unione dei backup vecchi…
phase-restoring = Ripristino in corso…
phase-verifying = Controllo in corso…
progress-of = { $done } di { $total }
skipped-count = { $count ->
    [one] 1 file saltato (in uso)
   *[other] { $count } file saltati (in uso)
}
profile-removed = Profilo "{ $name }" eliminato.
run-finished = "{ $name }" salvato: { $new } nuovi, { $changed } modificati, { $deleted } eliminati, { $added } copiati.
merge-finished = "{ $name }" unito: da { $before } a { $after } backup, { $freed } liberati.
restore-finished = { $count } file ripristinati.
verify-finished = { $count } file controllati, { $problems } problemi.
run-destination-missing = Destinazione non trovata: { $location }. Collega il disco e riprova.
run-busy = È in corso un altro backup o un'unione di questo profilo; riprova quando finisce.
run-failed = L'operazione non è riuscita: { $detail }
run-no-space = Spazio libero insufficiente sulla destinazione: servono { $needed }, liberi { $available }. Libera spazio, oppure unisci o elimina i backup vecchi, poi riavvialo: i file già copiati vengono tenuti e non ricopiati.
backup-resumed = { $count ->
    [one] 1 file ripreso dal backup interrotto
   *[other] { $count } file ripresi dal backup interrotto
}
run-cancelled = Annullato. I backup non sono stati modificati; un backup fermato riprende da dove si era interrotto la volta successiva.

# Errors
settings-unreadable = Il file delle impostazioni non si legge, quindi le modifiche non verranno salvate: { $detail }
save-failed = Impossibile salvare le impostazioni: { $detail }
schedule-failed = Impossibile pianificare l'esecuzione automatica: { $detail }
start-failed = Impossibile avviare l'operazione: { $detail }
unexpected-stop = L'operazione si è interrotta in modo inatteso.

# Restore browser
dates = Date dei backup
archive-tooltip = { $files } file, { $stored } salvati in questo backup ({ $size })
search-hint = Cerca un file per nome in tutti i backup…
pick-date = Scegli una data a sinistra.
tree-hint = Spunta file o cartelle per ripristinarli; fai clic su un file per vederne le versioni.
searching = Ricerca in corso…
no-results = Nessun file con questo nome.
results = { $count ->
    [one] 1 file trovato
   *[other] { $count } file trovati
}
version-count = { $count ->
    [one] 1 versione
   *[other] { $count } versioni
}
deleted-now = eliminato in seguito
close = Chiudi
versions-hint = Ogni versione è un contenuto diverso del file, dalla più recente.
version-modified = Modificato il { $date }
version-detail = Salvato dal backup del { $backup } · { $size }
restore-version = Ripristina questa versione
restore-to = Ripristina in:
target-beside = Accanto all'originale, con la data
target-beside-hint = relazione.docx viene ripristinato come "relazione (2026-11-02 21.30).docx": il file attuale non viene toccato.
target-original = Posizione originale
target-folder = Un'altra cartella
replace-existing = Sostituisci i file già presenti
replace-warning = I file attuali verranno sostituiti dalla copia del backup.
tick-files-hint = Spunta almeno un file o una cartella.
choose-folder-hint = Scegli la cartella in cui ripristinare.
restore-selected = Ripristina selezionati
selected-count = { $count ->
    [one] 1 elemento dal backup del { $date }
   *[other] { $count } elementi dal backup del { $date }
}
clear-selection = Togli selezione
tick-folder = Ripristina questa cartella e tutto il suo contenuto
tick-file = Ripristina questo file
show-versions = Mostra le versioni di questo file

# Manual merge
merge-choose = Scegli i backup da unire…
merge-choose-hint = Spunta il primo e l'ultimo backup: tutto quello che c'è in mezzo diventa un solo backup.
merge-tick = Spunta come estremo dell'intervallo da unire
merge-tick-hint = Spunta il backup più vecchio e quello più recente da unire; quelli in mezzo sono compresi.
merge-summary = { $count } backup diventano uno solo, con data { $date }.
merge-complete-warning = L'intervallo comprende il primo backup completo: verrà sostituito da un nuovo backup completo con la data del più recente scelto, e le date precedenti non saranno più ripristinabili.
merge-complete-ack = Ho capito
merge-two-hint = Spunta almeno due backup.
merge-ack-hint = Conferma di aver letto l'avviso.
merge-selected = Unisci

# Activity tab
tab-activity = Attività
show-details = Dettagli
cancel-hint = Ferma subito: i backup restano com'erano prima di questa esecuzione. Il backup successivo riusa i file già copiati.
activity-backup-running = Backup di "{ $name }" in corso
activity-merge-running = Unione dei backup di "{ $name }" in corso
activity-restore-running = Ripristino da "{ $name }" in corso
activity-backup-ended = Backup di "{ $name }"
activity-merge-ended = Unione dei backup di "{ $name }"
activity-restore-ended = Ripristino da "{ $name }"
activity-preparing = Preparazione…
activity-file = File
activity-folder = Cartella
activity-files = File
activity-speed = Velocità
activity-elapsed = Trascorso
activity-remaining = Rimanente
count-of = { $done } di { $total }
files-found = { $count } trovati finora
about = circa { $time }
activity-took = Durata: { $time }.
activity-backup-summary = Fatto: { $new } nuovi, { $changed } modificati, { $deleted } eliminati, { $unchanged } invariati; { $copied } copiati.
activity-merge-summary = Fatto: da { $before } a { $after } backup, { $freed } liberati.
activity-restore-summary = Fatto: { $restored } file ripristinati, { $kept } già presenti e lasciati.
show-backups = Mostra i backup
skipped-hint = Questi file non si sono potuti leggere, di solito perché un altro programma li tiene aperti. La loro versione precedente, se c'è, resta nei backup.
skipped-more = …e altri { $count } (vedi il registro).

# Exclusions and format lock
no-exclusions = Nessuna esclusione: viene salvato ogni file delle cartelle.
remove-exclusion = Torna a includerlo nel backup
exclusion-folder = Cartella e tutto il suo contenuto
exclusion-file = File
exclusion-type = Tutti i file di questo tipo
exclusion-pattern = Modello
exclude-folder = Escludi cartella…
exclude-file = Escludi file…
exclude-type-hint = .tmp
exclude-type = Escludi tipo
common-types = Comuni:
advanced = Avanzate
exclude-pattern = Escludi modello
container-locked = La destinazione contiene già backup in questo formato: non si può cambiare. Per usare l'altro, scegli una destinazione vuota.
container-unreachable = La destinazione non è raggiungibile, quindi il formato non si può cambiare adesso.
container-checking = Controllo della destinazione…
activity-idle-title = Nessuna operazione in corso
activity-idle-body = Qui vedi l'avanzamento di backup, unioni e ripristini.

# Backup kinds and deletion
menu-run-incremental = Backup incrementale ora
menu-run-complete = Backup completo ora
run-now = Esegui ora:
run-incremental = Incrementale
run-incremental-hint = Copia solo i file nuovi e modificati; è veloce ed è la scelta abituale.
run-complete = Completo
run-complete-hint = Copia tutti i file e inizia una nuova catena: i backup successivi non dipendono più dai precedenti. Richiede più tempo e spazio.
run-merge = Unione
run-first = Esegui il primo backup
run-incremental-button = Esegui backup incrementale
run-complete-button = Esegui backup completo
full-backup = Backup completo:
full-never = Solo il primo
full-every-backups = Ogni
full-every-days = Quando l'ultimo ha più di
full-backups-unit = backup
full-days-unit = giorni
full-backup-hint = Vale per i backup automatici; un backup completo inizia una nuova catena e rende il ripristino indipendente dai backup più vecchi.
run-finished-complete = "{ $name }" salvato per intero: { $new } nuovi, { $changed } modificati, { $deleted } eliminati, { $added } copiati.
activity-complete-summary = Backup completo fatto: { $new } nuovi, { $changed } modificati, { $deleted } eliminati, { $unchanged } invariati; { $copied } copiati.
activity-delete-running = Eliminazione di un backup di "{ $name }" in corso
activity-delete-ended = Backup eliminato da "{ $name }"
activity-delete-summary = Fatto: da { $before } a { $after } backup, { $freed } liberati.
delete-finished = Un backup di "{ $name }" è stato eliminato.
delete-backup = Elimina questo backup…
delete-backup-hint = Toglie la data selezionata; i backup successivi restano completi.
delete-backup-confirm = Elimina backup
delete-initial-hint = Il primo backup completo si può eliminare solo quando è l'unico: gli altri dipendono da lui.
delete-complete-hint = Un backup completo inizia una catena da cui dipendono i backup successivi: non si può eliminare. Puoi unirlo ai successivi.
delete-only-warning = Il backup del { $date } è l'unico: la destinazione resterà vuota e il prossimo backup sarà completo. L'operazione non si può annullare.
delete-newest-warning = Il backup del { $date } verrà eliminato, con le versioni dei file salvate solo lì. L'operazione non si può annullare.
delete-middle-warning = Il backup del { $date } verrà eliminato. I file che servono ai backup successivi passano al backup seguente; le versioni salvate solo in questo vanno perse. L'operazione non si può annullare.

# Encryption
encrypt = Cifra i backup (AES-256, anche i nomi dei file)
encrypt-hint = Ogni file diventa uno zip AES-256 con un nome nascosto: senza la password nessuno può leggerne il contenuto né vedere quali file sono salvati. La password è conservata in Gestione credenziali di Windows.
encrypt-needs-zip = La cifratura richiede il formato "Uno zip per file".
password = Password
password-again = Ripeti la password
password-set = Scegli la password
password-reenter = Inserisci la password di questi backup
password-saved = Password salvata in Gestione credenziali di Windows.
password-missing = Su questo computer non c'è una password salvata: inseriscila per fare i backup e sfogliarli.
password-loss-warning = Senza questa password, o la chiave di recupero mostrata dopo il primo backup, i backup non si possono aprire e nessuno può recuperarli. Conserva la password in un posto sicuro.
password-save = Salva password
password-checking = Verifica della password…
password-empty-hint = Scrivi la password.
password-short-hint = Usa almeno { $count } caratteri.
password-mismatch-hint = Le due password sono diverse.
password-first-hint = Salva prima la password dei backup cifrati.
password-wrong = Questa password non apre i backup della destinazione.
password-unreachable = Non è stato possibile verificare la password
run-password-needed = I backup sono cifrati e su questo computer non c'è una password salvata: inseriscila nelle impostazioni del profilo.
run-wrong-password = La password salvata non apre i backup: inseriscila di nuovo nelle impostazioni del profilo.

# Backup contents
tree-count = { $stored } di { $total } file salvati da questo backup
tree-stored-hint = I file salvati da questo backup (nuovi o modificati) sono in evidenza; quelli in grigio sono stati salvati da backup precedenti e non sono cambiati, e si possono ripristinare lo stesso. Spunta per ripristinare; una cartella spuntata torna per intero, com'era a quella data. Fai clic su un file per vederne le versioni.
tree-nothing-stored = Questo backup non ha salvato nulla: nessun file era cambiato.
tree-not-saved-folder = In questa cartella non è cambiato nulla in questo backup.
tree-not-saved-file = Non cambiato in questo backup: il contenuto è in uno precedente. Fai clic per vederne le versioni.

# Operations log
log-runs = Operazioni
log-only-profile = Solo questo profilo
log-running = In corso
log-last = Ultima operazione
log-empty = Ancora nessuna operazione.
log-pick = Scegli un'operazione a sinistra.
log-title = Registro
log-no-lines = Ancora niente da registrare.
log-scheduled = automatica
log-manual = avviata a mano
log-started = Avviata il { $time }
log-backup = backup { $name }
log-unfinished = Non terminata: interrotta, oppure ancora in corso.
log-skipped = Saltato { $path }: { $reason }
log-op-backup = Backup
log-op-incremental = Backup incrementale
log-op-complete = Backup completo
log-op-merge = Unione
log-op-delete = Eliminazione di un backup
log-op-restore = Ripristino

# Password change and recovery key
password-change = Cambia la password
password-new = Nuova password
password-change-button = Cambia password
recovery-later = La chiave di recupero sarà disponibile dopo il primo backup.
recovery-show = Mostra la chiave di recupero…
recovery-title = Chiave di recupero
recovery-hint = Con questa chiave 7-Zip apre ogni file cifrato e ogni elenco dei file, anche senza Archeion e senza password. L'elenco di ciascun backup indica dove è salvato ogni file.
recovery-warning = Chi ha questa chiave può leggere tutti i backup. Stampala o scrivila e conservala offline; non cambia quando cambi la password.
recovery-copy = Copia
recovery-copy-hint = Copia la chiave senza spazi, come la vuole 7-Zip.
recovery-hide = Nascondi

# Integrity check
verify-section = Controlli
verify-hint = Il controllo completo rilegge ogni file salvato in ogni backup e lo confronta con la sua impronta; la prova di ripristino rilegge alcuni file a caso del backup più recente. I controlli dovuti partono dopo un backup pianificato.
verify-full-every = Controllo completo ogni
verify-sample-every = Prova di ripristino ogni
verify-sample-files = su file:
verify-zero-hint = 0 giorni = mai in automatico.
verify-full-now = Controlla i backup ora
verify-full-hint = Rilegge e verifica ogni file salvato; su una destinazione grande richiede il tempo di leggere tutti i backup.
verify-sample-now = Prova di ripristino ora
verify-sample-hint = Rilegge alcuni file a caso del backup più recente, come farebbe un ripristino, senza scriverli da nessuna parte.
verify-no-backups = Non ci sono ancora backup da controllare.
verify-problems = Problemi trovati
verify-problems-hint = I backup indicati potrebbero non ripristinarsi correttamente. Esegui subito un backup completo e controlla il disco di destinazione.
activity-verify-running = Controllo dei backup di "{ $name }" in corso
activity-verify-ended = Controllo dei backup di "{ $name }"
log-op-verify = Controllo di integrità
log-op-restore-test = Prova di ripristino
