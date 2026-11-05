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
exclusions-hint = Un modello vale sul percorso completo, senza distinguere maiuscole e minuscole: * qualsiasi testo in un nome, ** qualsiasi cartella, ? un carattere. Per esempio **/node_modules, **/cache/**, **/~$*.
destination = Destinazione
dest-local = Cartella o disco esterno
dest-network = Cartella di rete
dest-hint = Usa una cartella riservata a questo profilo. Se la cifratura (qui sotto) non è attiva, chi può aprirla può leggere i backup.
dest-free = Spazio libero sulla destinazione: { $free }
dest-fs-exfat = Questa unità usa exFAT, che non ha un journal: un'interruzione di corrente o l'unità scollegata durante un backup o un merge possono danneggiare i backup. NTFS è più sicuro.
dest-fs-fat = Questa unità usa FAT32: non può contenere file di 4 GiB o più, e non ha un journal, quindi un'interruzione di corrente durante un backup o un merge può danneggiare i backup. Formattala in NTFS, o exFAT.
dest-foreign-profile = Questa cartella contiene i backup del profilo "{ $name }": scegli un'altra cartella per questo profilo.
dest-foreign-deleted = Questa cartella contiene i backup di un profilo che non esiste più.
dest-adopt = Usa questi backup per questo profilo
dest-adopt-hint = I backup esistenti diventano di questo profilo: il prossimo backup li continua, e da qui si possono ripristinare e unire.
dest-adopt-failed = Impossibile prendere in carico i backup: { $detail }
dest-foreign-hint = La destinazione contiene i backup di un altro profilo: vedi la sezione Destinazione.
run-foreign-destination = La destinazione contiene i backup di un altro profilo. Apri le impostazioni del profilo: se quel profilo è stato eliminato, "Usa questi backup per questo profilo" li prende in carico.
run-encryption-missing = I backup nella destinazione non risultano cifrati, ma il profilo o i backup stessi lo sono: o la cifratura è stata attivata dopo i primi backup, o la descrizione della destinazione è stata tolta o cambiata fuori da Archeion. Non è stato scritto nulla. Per i backup cifrati scegli una destinazione nuova e vuota.
network-unconfirmed = Questo profilo è stato importato e usa cartelle su altri computer ({ $hosts }). Archeion non li ha ancora contattati: Windows vi accederebbe con il tuo account. Controlla di conoscere questi computer, poi conferma.
network-confirm = Usa queste cartelle di rete
run-network-unconfirmed = Questo profilo importato usa cartelle su altri computer non ancora confermate: apri le sue impostazioni e premi «Usa queste cartelle di rete».
browse = Sfoglia…
container = Formato
container-zip-per-file = Uno zip per file
container-folder = Copie semplici
container-zip-per-file-hint = Ogni file è compresso da solo (relazione.docx diventa relazione.docx.zip) nelle stesse cartelle; si apre con Esplora file o 7-Zip.
container-folder-hint = I file sono copiati così come sono, con gli stessi nomi, senza compressione.
large-files = Salva i file grandi solo come modifiche, da
large-files-unit = MB
large-files-hint = I file di almeno questa dimensione sono salvati a pezzi: dopo una modifica, un backup memorizza solo i pezzi cambiati. I backup completi li conservano anche interi, così si aprono ancora con 7-Zip. I file più piccoli sono salvati come sopra.
large-files-change-hint = Puoi cambiarla quando vuoi: i backup successivi seguono la nuova soglia, quelli già fatti restano come sono.
large-files-encrypted = Ogni file è salvato a pezzi, così di un file modificato si memorizza solo ciò che è cambiato; i backup completi conservano anche le copie intere.
schedule = Backup automatico
schedule-manual = Solo quando lo avvio io
at = alle
schedule-hint = Se a quell'ora il computer è spento, parte appena si riaccende.
schedule-paused = Questo profilo è stato importato: la sua pianificazione non è ancora attiva. Controlla cosa salva e dove, poi attivala.
schedule-resume = Attiva la pianificazione
merge = Unione dei backup vecchi
merge-hint = I backup vecchi non vengono cancellati ma uniti al successivo che resta: ogni data conservata rimane completa e lo spazio si libera. Il primo backup completo non viene mai unito in automatico.
merge-automatic-hint = L'unione è automatica: mentre Archeion è in esecuzione unisce i backup vecchi secondo le regole qui sotto, mai mentre un backup o un'altra operazione li sta usando.
merge-keep-deleted = Mantieni nel backup unito i file eliminati dall'origine
merge-keep-deleted-hint = Un file eliminato tra i backup che si uniscono resta nel backup unito con il suo ultimo contenuto, così si può ancora ripristinare. Togli la spunta per scartarlo e liberare il suo spazio.
retention-hint = Un backup resta se almeno una regola lo sceglie. I backup degli ultimi giorni indicati in "Tutti, per giorni" non vengono mai uniti.
merge-preview = { $count ->
    [one] Con queste regole la prossima unione unisce 1 backup ({ $from }) al successivo che resta.
   *[other] Con queste regole la prossima unione unisce { $count } backup (dal { $from } al { $to }) ai successivi che restano.
}
merge-preview-none = Con queste regole ora non c'è niente da unire.
merge-waits = { $count ->
    [one] Le nuove regole unirebbero 1 backup che con quelle precedenti restava: l'unione automatica aspetta la tua conferma.
   *[other] Le nuove regole unirebbero { $count } backup che con quelle precedenti restavano: l'unione automatica aspetta la tua conferma.
}
merge-confirm = Unisci con le nuove regole
merge-confirm-hint = L'unione non si può annullare: le versioni di quei backup vanno perse.
merge-restore-rules = Ripristina le regole precedenti
keep-within = Tutti, per giorni
keep-last = Più recenti
keep-daily = Uno al giorno, per giorni
keep-weekly = Uno a settimana, per settimane
keep-monthly = Uno al mese, per mesi
keep-yearly = Uno all'anno, per anni
keep-everything = Tutte le regole a 0: ogni backup resta e non si unisce nulla.
merge-now = Unisci ora
merge-now-hint = Applica subito le regole di unione; il primo backup completo resta com'è.
busy-hint = Un backup o un'unione è già in corso.
fix-profile-hint = Completa prima il profilo.
delete-profile = Elimina profilo
delete-profile-hint = Chiede conferma, poi toglie il profilo e le sue attività pianificate; i backup restano nella destinazione, a meno che tu non scelga di eliminare anche quelli.
profile-item-hint = Trascina per cambiare l'ordine; tasto destro per rinominare o eliminare.
rename-profile = Rinomina
move-profile-up = Sposta su
move-profile-down = Sposta giù
move-profile-first = È già il primo.
move-profile-last = È già l'ultimo.
delete-profile-menu = Elimina profilo…
key-delete = Canc
delete-profile-title = Eliminare il profilo "{ $name }"?
delete-profile-body = Il profilo e le sue attività pianificate vengono tolti. Finché non chiudi il messaggio nella barra di stato, "Annulla eliminazione" li ripristina.
delete-profile-password = La password salvata viene tolta da Gestione credenziali di Windows quando l'eliminazione non si può più annullare: chiudendo quel messaggio, eliminando un altro profilo o uscendo da Archeion.
delete-profile-backups = I backup in { $path } non vengono cancellati: un nuovo profilo che usa quella cartella può prenderli in carico con "Usa questi backup per questo profilo".
delete-profile-with-backups = Sposta anche i backup in { $path } nel Cestino
delete-profile-backups-gone = Si possono ripristinare dal Cestino finché non lo svuoti, ma "Annulla eliminazione" non potrà più ripristinare il profilo. Va via solo ciò che ha scritto Archeion: gli altri file della cartella restano. Una cartella di rete o un disco rimovibile non hanno il Cestino: lì non viene spostato niente.
delete-profile-and-backups = Elimina profilo, backup nel Cestino
delete-profile-body-backups = Il profilo e le sue attività pianificate vengono eliminati, e i suoi backup vanno nel Cestino.
delete-backups-running = Spostamento dei backup di "{ $name }" nel Cestino…
delete-backups-done = I backup di "{ $name }" sono nel Cestino.
delete-backups-left = I backup di "{ $name }" sono nel Cestino; { $path } resta, con { $count ->
    [one] 1 elemento
   *[other] { $count } elementi
} non scritti da Archeion.
delete-backups-failed = Non tutti i backup di "{ $name }" sono potuti andare nel Cestino: { $detail }
delete-backups-no-bin = { $path } non ha il Cestino (cartella di rete o disco rimovibile): i backup di "{ $name }" non sono stati toccati. Eliminali a mano se non ti servono più.

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
restore-no-space = Spazio libero insufficiente dove vengono ripristinati i file: servono { $needed }, liberi { $available }. Non è stato scritto nulla: libera spazio, oppure ripristina meno file o su un altro disco.
backup-resumed = { $count ->
    [one] 1 file ripreso dal backup interrotto
   *[other] { $count } file ripresi dal backup interrotto
}
backup-as-changes = { $count ->
    [one] 1 file salvato solo come modifiche
   *[other] { $count } file salvati solo come modifiche
}
backup-efs-clear = { $count ->
    [one] 1 file cifrato da Windows (EFS) è stato salvato senza quella cifratura
   *[other] { $count } file cifrati da Windows (EFS) sono stati salvati senza quella cifratura
}
backup-efs-clear-hint = Per tenerli protetti, attiva la cifratura su una nuova destinazione di questo profilo.
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
search-hint = Cerca un file o una cartella per nome in tutti i backup…
include-deleted = Includi i file eliminati nelle cartelle spuntate
include-deleted-hint = Attiva: una cartella spuntata riporta anche i file eliminati dall'origine nel frattempo, con _deleted_ davanti al nome. Spenta: torna com'era alla sua data; un file eliminato si può comunque spuntare da solo.
no-results = Nessun file o cartella con questo nome.
results = { $count ->
    [one] 1 file trovato
   *[other] { $count } file trovati
}
close = Chiudi
restore-to = Ripristina in:
target-beside = Accanto all'originale, con la data
target-beside-hint = relazione.docx viene ripristinato come "relazione (2026-11-02 21.30).docx": il file attuale non viene toccato.
target-original = Posizione originale
target-folder = Un'altra cartella
replace-existing = Sostituisci i file già presenti
replace-warning = I file attuali verranno sostituiti dalla copia del backup.
tick-files-hint = Spunta almeno un file o una cartella.
choose-folder-hint = Scegli la cartella in cui ripristinare.
restore-selected = Ripristina
clear-selection = Togli selezione
tick-folder = Ripristina questa cartella e tutto il suo contenuto
tick-file = Ripristina questo file
backups-restore-files = Ripristina file
backups-restore-files-hint = Ogni file mai salvato: spunta ciò che vuoi recuperare, scegli le versioni o la data e dove va.
backups-manage = Gestisci i backup
backups-manage-hint = L'elenco delle date dei backup: unisci più backup in uno o eliminane uno.
restore-tree-count = { $files ->
        [one] 1 file
       *[other] { $files } file
    } nei backup · eliminati dall'origine: { $deleted }
restore-tree-hint = Spunta file o cartelle per ripristinarli; la ricerca trova un file o una cartella per nome, con maiuscole o minuscole. I file segnati { $deleted-icon } eliminato non sono più nell'origine e si possono comunque ripristinare.
deleted-label = eliminato
excluded-label = escluso
tree-excluded-file = Tolto da un'esclusione aggiunta dopo i suoi backup: le versioni salvate restano. Una cartella spuntata non lo riporta; spuntalo da solo: torna con la data nel nome.
tree-excluded-folder = Tolta da un'esclusione aggiunta dopo i suoi backup: le versioni salvate restano. Spunta ciò che contiene, uno per uno, per riportarlo.
tick-excluded-in-folder = Escluso dalla cartella spuntata { $folder } da un'esclusione aggiunta dopo. Spuntalo per ripristinare anche lui, con la data nel nome.
tree-deleted-file = Non è nel backup più recente: eliminato dall'origine. Spuntalo per ripristinarne una versione.
tree-deleted-folder = Non è nel backup più recente: eliminata dall'origine. Spuntala per ripristinarla com'era in una data.
tick-in-folder = Ripristinato con la cartella spuntata { $folder }. Togli la spunta per escluderlo.
tick-not-in-folder-date-folder = Non è nel backup con cui torna la cartella spuntata { $folder }: non viene ripristinato. Togli la spunta a quella cartella per sceglierlo da solo.
tick-not-in-folder-date = Non è nel backup con cui torna la cartella spuntata { $folder } (eliminato dall'origine prima). Spuntalo per ripristinare anche lui, nella sua ultima versione o in quella scelta.
tick-partial = Dentro c'è qualcosa di spuntato, o di escluso dalla cartella spuntata. Fai clic per spuntare tutta la cartella.
tick-full = Tutto ciò che contiene è spuntato elemento per elemento. Fai clic per togliere tutte le spunte.
tick-left-out = Escluso dalla cartella spuntata { $folder }: non viene ripristinato. Spuntalo per ripristinarlo con la cartella.
selection-empty = Ancora niente da ripristinare. Spunta file o cartelle nell'albero.
file-name-hint = Fai clic per vedere le sue versioni e scegliere quella da ripristinare.
versions-hint = Scegli la versione da ripristinare; senza una scelta, la più recente.
version-detail = Backup del { $backup } · { $size }
version-modified = Modificato il { $date }
versions-in-folder = Torna con la cartella spuntata { $folder }, com'era in quel backup. Scegli un'altra versione per ripristinarlo da solo.
versions-tick-hint = Spunta il file nell'albero per ripristinare la versione scelta.
restore-to-hint = Dove va tutto ciò che è spuntato.
restore-conflict = Due elementi spuntati sostituirebbero lo stesso file originale: togli la spunta a uno dei due.
restore-conflict-hint = Due elementi spuntati sostituirebbero lo stesso file originale: togli la spunta a uno dei due.
restore-wait-versions = Lettura delle versioni dei file spuntati…
restore-summary = { $files ->
        [one] 1 file
       *[other] { $files } file
    }, { $folders ->
        [one] 1 cartella
       *[other] { $folders } cartelle
    } — { $size }

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
exclude-type = Escludi tipo
exclude-pattern = Escludi modello
exclude-rule-hint = **/node_modules · .iso
rule-empty = Scrivi un modello o un tipo di file, poi premi il pulsante che dice quale dei due è.
rule-type-not-pattern = «{ $text }» è un tipo di file: premi «Escludi tipo».
rule-pattern-not-type = «{ $text }» è un modello: premi «Escludi modello».
rule-name-alone = Un modello vale sul percorso intero, quindi «{ $text }» da solo non esclude nulla. Per una cartella o un file con questo nome, ovunque sia, scrivi **/{ $text }.
rule-bad-type = «{ $text }» non è un tipo di file: scrivi solo l'estensione, senza spazi (per esempio .iso).
rule-bad-pattern = «{ $text }» non è un modello valido: { $detail }.
rule-already = Questa esclusione è già nell'elenco.
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
password-short-hint = Usa da { $count } a { $max } caratteri.
password-mismatch-hint = Le due password sono diverse.
password-rule-length = Da { $count } a { $max } caratteri (ora { $typed })
password-rule-match = Le due password coincidono
password-rule-common = Non è una password comune, come "Password2024" o "qwerty"
password-rule-runs = Nessuna sequenza di { $count } caratteri uguali o consecutivi, come "aaaa", "1234" o "abcd"
password-rule-name = Non contiene il nome del profilo
password-rule-strength = Abbastanza difficile da indovinare: robustezza almeno { $level }
password-rule-tip = Va bene qualunque carattere, spazi compresi: una frase di quattro o cinque parole si ricorda facilmente ed è difficile da indovinare. Non servono cifre, maiuscole o simboli.
password-strength = Robustezza: { $level }
password-strength-weak = Debole
password-strength-fair = Discreta
password-strength-strong = Buona
password-strength-very-strong = Ottima
password-common-hint = È una delle password più usate: scegline un'altra.
password-runs-hint = Evita sequenze come "aaaa" o "1234".
password-name-hint = Non mettere il nome del profilo nella password.
password-weak-hint = Troppo facile da indovinare: allungala, per esempio con qualche parola in più.
password-first-hint = Scrivi qui sopra la password dei backup cifrati (due volte per backup nuovi, e spunta "Ho capito"): il pulsante la salva e avvia il backup.
password-wrong = Questa password non apre i backup della destinazione.
password-unreachable = Non è stato possibile verificare la password
run-password-needed = I backup sono cifrati e su questo computer non c'è una password salvata: inseriscila nelle impostazioni del profilo.
run-wrong-password = La password salvata non apre i backup: inseriscila di nuovo nelle impostazioni del profilo.

# Operations log
log-runs = Operazioni
log-only-profile = Solo questo profilo
log-running = In corso
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
log-open-full = Apri il registro completo
log-open-full-hint = Scrive l'intero registro di questa operazione in un file di testo e lo apre nell'editor di testo: ogni passo, poi ogni file su cui ha lavorato e che cosa ne ha fatto, compresi i file saltati.
log-copy = Copia il registro
log-copy-hint = Copia l'intero registro di questa operazione come testo, da incollare in un'email o in una segnalazione.
log-copied = Registro copiato.
log-open-program = Registro del programma
log-open-program-hint = Apre archeion.log, il registro tecnico del programma: utile per indagare su un problema o per segnalarlo.
log-open-folder = Cartella dei registri
log-open-folder-hint = Apre in Esplora file la cartella dei registri delle operazioni.
log-program-missing = Il registro del programma non esiste ancora.
log-unreadable = Il file di registro non è leggibile.
log-report-failed = Impossibile mostrare il registro completo: { $detail }
log-level-info = Info
log-level-done = OK
log-level-warning = Avviso
log-level-error = Errore
log-preparing = Preparazione del registro completo…
log-files-title = File
log-files-missing = L'elenco dei file non è stato registrato per questa operazione: è stata eseguita da una versione precedente di Archeion.
log-files-none = Nessun file elaborato.
log-file-new-chunks = salvato come modifiche: scritti { $size } di blocchi nuovi
log-file-resumed = già copiato dal backup interrotto, mantenuto
log-count-new = Nuovi: { $count }
log-count-changed = Modificati: { $count }
log-count-as-changes = Salvati come modifiche: { $count }
log-count-copied-again = Copiati di nuovo: { $count }
log-count-unchanged = Invariati, non elencati: { $count }
log-count-deleted = Eliminati: { $count }
log-count-moved = Spostati: { $count }
log-count-dropped = Versioni rimosse: { $count }
log-count-restored = Ripristinati: { $count }
log-count-kept = Mantenuti: { $count }
log-count-checked = Riletti integri: { $count }
log-count-checked-unlisted = Riletti integri, non elencati: { $count }
log-count-damaged = Problemi: { $count }
log-count-skipped = Saltati: { $count }

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
password-current = Password attuale
password-current-hint = Scrivi la password attuale.
recovery-ask = Per mostrare la chiave di recupero, scrivi la password attuale di questi backup.
recovery-show-button = Mostra

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
log-op-verify = Controllo di integrità
log-op-restore-test = Prova di ripristino

# Automatic backup: schedules, events, notifications
schedule-days = Nei giorni scelti
schedule-hours = Ogni tot ore
schedule-monthly = Una volta al mese
schedule-all-days = Tutti i giorni
schedule-add-time = + Aggiungi un orario
schedule-remove-time = Togli questo orario
schedule-every = Ogni
hours-unit = ore
schedule-from = dalle
schedule-until = alle
schedule-hours-hint = L'ultima esecuzione parte prima dell'ora di fine; stessa ora di inizio e di fine = tutto il giorno.
schedule-month-day = Il giorno
schedule-month-hint = Solo dal giorno 1 al 28, così ogni mese ce l'ha.
alerts = Notifiche
alert-on-failure = Avvisami quando un backup automatico non riesce
alert-stale = Avvisami quando non c'è un backup da
alert-hint = Notifiche di Windows; 0 giorni = nessun avviso. Il controllo avviene ogni giorno e dopo l'accesso.
alert-manual-hint = Solo per i backup automatici: scegli una pianificazione in "Backup automatico".
alert-failed-title = Archeion: il backup di "{ $name }" non è riuscito
alert-stale-title = Archeion: "{ $name }" senza backup
alert-stale-body = Nessun backup da { $days } giorni (l'ultimo il { $when }). Controlla che la destinazione sia collegata, oppure avvia un backup da Archeion.
alert-unusual-title = Archeion: modifiche insolite in "{ $name }"
alert-unusual-body = { $count } file sono nuovi, cambiati o eliminati dall'ultimo backup, molti più del solito. Se non te lo aspettavi, controlla i tuoi file: le versioni precedenti sono ancora nei backup.
alert-went-back-title = Archeion: backup più vecchi nella destinazione di "{ $name }"
alert-went-back-body = I backup più recenti visti in precedenza non ci sono più: la destinazione è stata rimessa da una copia vecchia, o dei backup sono stati tolti fuori da Archeion. Se non l'hai fatto tu, controlla la destinazione.
alert-changed-outside-title = Archeion: backup cambiati nella destinazione di "{ $name }"
alert-changed-outside-body = Dei backup sono stati tolti o rimessi nella destinazione fuori da Archeion: il loro elenco non corrisponde più a quello firmato da Archeion. Se non l'hai fatto tu, controlla la destinazione.
run-changed-outside = dei backup sono stati tolti o rimessi nella destinazione fuori da Archeion: se non l'hai fatto tu, controlla la destinazione
closing-after-run = Un'operazione è in corso: Archeion si chiuderà da solo quando sarà finita.
closing-cancel = Non chiudere
shutdown-reason = È in corso un'operazione di backup: se Windows si spegne ora, si ferma e riprende la volta successiva.

# Area di notifica e finestra Preferenze
menu-preferences = Preferenze…
preferences-title = Preferenze
preferences-window = Finestra
language = Lingua
language-system = Lingua di sistema
language-tray-restart = Il menu dell'icona nell'area di notifica cambia lingua al prossimo avvio di Archeion.
quit-hint = Chiude Archeion; i backup pianificati continuano a partire dall'Utilità di pianificazione di Windows.
tray-open = Apri Archeion
tray-quit = Esci
tray-running = operazione in corso
option-tray = Resta attivo nell'area di notifica quando chiudi la finestra
option-tray-hint = Chiudendo la finestra, Archeion si nasconde accanto all'orologio: continua a unire i backup vecchi e a finire l'operazione in corso. Fai clic sulla sua icona per riaprirlo, oppure scegli Esci.
option-tray-unavailable = Windows non ha fornito un'icona nell'area di notifica.
option-autostart = Avvia Archeion con Windows
option-autostart-hint = Quando accedi a Windows, Archeion parte nascosto nell'area di notifica. Puoi disattivarlo anche da Gestione attività, in App di avvio.
autostart-failed = Non è stato possibile cambiare l'avvio con Windows: { $detail }

# Help menu: user guide and About
menu-documentation = Documentazione
menu-about = Informazioni su Archeion
about-version = Versione { $version }
about-name = Il nome viene dal greco antico ἀρχεῖον, "archivio".
about-author = Autore: MR
about-license = Licenza: GPL-3.0
about-repository = Codice sorgente su GitHub
about-logs-folder = Cartella dei log
about-open-folder = Apri cartella
about-folder-missing = La cartella non esiste ancora.
doc-title = Documentazione di Archeion
doc-intro = Archeion conserva copie delle tue cartelle su un altro disco, una cartella di rete o qualsiasi altra cartella, e ti restituisce ogni file com'era in una data passata. Apri una sezione per leggerla.
doc-what-title = Che cosa fa Archeion
doc-what-backups = Il primo backup di un profilo è completo: copia tutti i file. I successivi sono incrementali: copiano solo i file nuovi e modificati, quindi sono veloci e occupano poco, ma ogni data si può ripristinare per intero. In "Backup completo", nelle impostazioni del profilo, puoi anche chiedere un nuovo backup completo ogni tot backup, o quando l'ultimo è più vecchio di un certo numero di giorni; un backup completo inizia una nuova catena che non dipende più dai backup precedenti.
doc-what-storage = Ogni file è salvato da solo, nelle stesse cartelle dell'originale: come zip di quel file ("Uno zip per file": relazione.docx diventa relazione.docx.zip), come copia semplice o come zip cifrato AES-256. I backup si leggono anche senza Archeion: Esplora file e 7-Zip aprono gli zip, e 7-Zip apre quelli cifrati con la chiave di recupero.
doc-what-safe = Un backup interrotto (computer spento, disco scollegato, Annulla) lascia i backup esistenti com'erano; il backup successivo riprende da dove si era fermato e non ricopia i file già copiati.
doc-profiles-title = Profili
doc-profiles-body = Un profilo dice che cosa salvare, dove e quando. Creane uno con File > Nuovo profilo (Ctrl+N) e compila la scheda Impostazioni: un nome, le cartelle da salvare e la destinazione, cioè una cartella o un disco esterno oppure una cartella di rete. Usa ogni cartella di destinazione per un solo profilo.
doc-profiles-list = L'elenco dei profili mantiene l'ordine che gli dai: trascina un profilo in alto o in basso, oppure usa Sposta su e Sposta giù (Alt+Su, Alt+Giù). Il tasto destro su un profilo apre i suoi comandi, che sono anche nel menu File: Rinomina (F2) cambia il nome sul posto, Invio lo conferma ed Esc annulla; Elimina profilo… (Canc) chiede conferma.
doc-profiles-exclusions = Le esclusioni lasciano fuori ciò che non serve: una cartella con tutto il suo contenuto, un file, tutti i file di un tipo come .iso («Escludi tipo»), o ciò che corrisponde a un modello («Escludi modello»): i modelli valgono sul percorso completo senza distinguere maiuscole e minuscole, * è qualsiasi testo, ** qualsiasi cartella, ? un carattere, per esempio **/node_modules o **/cache/**. Scrivi la regola nel campo e premi il pulsante che dice cos'è: il testo viene controllato. I profili nuovi partono con .tmp, .temp e le cartelle tmp e temp escluse. Un file escluso dopo i suoi backup conserva le versioni salvate: ripristinando una cartella resta fuori, spuntato da solo torna con la data nel nome.
doc-profiles-format = Il formato di salvataggio (uno zip per file o copie semplici) e la cifratura vanno scelti prima del primo backup: quando la destinazione contiene dei backup non si possono più cambiare. Per passare all'altro, scegli una destinazione vuota.
doc-profiles-delete = Elimina profilo chiede conferma, poi toglie il profilo e le sue attività pianificate; i backup restano nella destinazione. Subito dopo, "Annulla eliminazione" nella barra di stato ripristina il profilo. Un profilo che in seguito usa quella cartella può prenderli in carico con "Usa questi backup per questo profilo". Nella conferma, "Sposta anche i backup nel Cestino" sposta anche quelli nel Cestino, senza "Annulla eliminazione" per il profilo: va via solo ciò che Archeion ha scritto nella cartella, e la cartella intera quando non contiene altro. Una cartella di rete o un disco rimovibile non hanno il Cestino: lì non viene spostato niente.
doc-auto-title = Backup automatici
doc-auto-schedule = In "Backup automatico" scegli quando i backup partono da soli: nei giorni scelti a uno o più orari, ogni tot ore tra due orari, oppure una volta al mese in un giorno dall'1 al 28. Archeion registra la pianificazione nell'Utilità di pianificazione di Windows, così i backup partono anche quando Archeion non è aperto.
doc-auto-missed = Se all'ora prevista il computer è spento, il backup parte appena si riaccende.
doc-auto-alerts = In "Notifiche" Archeion può mostrare una notifica di Windows quando un backup automatico non riesce, e un avviso quando un profilo non ha backup da un certo numero di giorni (il controllo avviene ogni giorno e dopo l'accesso).
doc-auto-now = Puoi sempre avviare un backup tu: Esegui backup nelle impostazioni del profilo, oppure Backup > Backup incrementale ora (Ctrl+B) e Backup completo ora.
doc-merge-title = Unione dei backup vecchi
doc-merge-body = I backup vecchi non vengono cancellati ma uniti al successivo che resta: ogni data conservata rimane completa e si libera lo spazio delle versioni che non servono più. L'unione è automatica: avviene mentre Archeion è in esecuzione, mai mentre un backup o un'altra operazione sta usando la destinazione. Il backup unito mantiene i file eliminati dall'origine tra i backup che fonde, con il loro ultimo contenuto, a meno che tu non tolga la spunta a "Mantieni nel backup unito i file eliminati dall'origine": nelle impostazioni del profilo per l'unione automatica, in Gestisci i backup per un'unione a mano.
doc-merge-rules = Le regole in "Unione dei backup vecchi" dicono quali backup tenere, e un backup resta se almeno una regola lo sceglie: tutti i backup degli ultimi giorni, i più recenti, uno al giorno, uno a settimana, uno al mese e uno all'anno, ciascuno per il numero che imposti. Con tutte le regole a 0 ogni backup resta e non si unisce nulla.
doc-merge-confirm = Regole che unirebbero backup che con quelle precedenti restavano non si applicano da sole: l'unione automatica aspetta che tu scelga "Unisci con le nuove regole" o "Ripristina le regole precedenti" (Unisci ora vale come un sì). Sotto le regole, una riga dice che cosa unirebbe la prossima unione.
doc-merge-first = Il primo backup completo non viene mai unito in automatico.
doc-merge-manual = In Backup > Gestisci i backup, "Scegli i backup da unire…" unisce a mano: spunta il backup più vecchio e quello più recente, e tutto quello che c'è in mezzo diventa un solo backup. Se l'intervallo comprende il primo backup completo, Archeion avvisa prima, perché le date precedenti non saranno più ripristinabili.
doc-merge-delete = "Elimina questo backup…" in Backup > Gestisci i backup toglie una data; i backup successivi restano completi. Un backup completo da cui dipendono i successivi non si può eliminare (uniscilo invece ai successivi). L'eliminazione non si può annullare.
doc-encrypt-title = Cifratura
doc-encrypt-body = Con "Cifra i backup" ogni file diventa uno zip AES-256 con un nome nascosto: senza la password nessuno può leggerne il contenuto né vedere quali file sono salvati. La cifratura richiede il formato "Uno zip per file" e va attivata prima del primo backup; dopo non si può più cambiare.
doc-encrypt-password = La password è salvata in Gestione credenziali di Windows, così i backup automatici non chiedono nulla. Puoi cambiarla in seguito nelle impostazioni del profilo, scrivendo quella attuale. Su un altro computer basta scriverla una volta nelle impostazioni del profilo.
doc-encrypt-recovery = Dopo il primo backup, "Mostra la chiave di recupero…" mostra una chiave con cui 7-Zip apre ogni file cifrato, anche senza Archeion e senza la password. Stampala o scrivila e conservala lontano dal computer: senza la password e senza la chiave nessuno può aprire i backup. La chiave non cambia quando cambi la password. Archeion chiede la password attuale prima di mostrarla.
doc-large-title = File grandi
doc-large-body = Un file grande che cambia di poco (una casella di posta, il disco di una macchina virtuale, un database) non viene ricopiato per intero: è salvato in pezzi di circa 1 MB e ogni backup memorizza solo i pezzi nuovi. I pezzi identici sono memorizzati una volta sola. Vale per i file di almeno la dimensione scelta nella sezione Formato del profilo (500 MB se non la cambi; si può disattivare) e per tutti i file di una destinazione cifrata.
doc-large-complete = I backup completi conservano comunque ogni file intero, come zip, copia semplice o zip cifrato, così si possono ripristinare con il solo 7-Zip; solo i backup incrementali che li seguono contengono pezzi. Il ripristino rimette insieme i pezzi e controlla ognuno; il controllo di integrità legge ogni pezzo.
doc-large-upgrade = I backup fatti dalle versioni precedenti restano come sono e si leggono ancora. I pezzi iniziano con il backup completo successivo: in una destinazione così il prossimo backup automatico è completo.
doc-restore-title = Ripristino dei file
doc-restore-body = Apri la scheda Backup del profilo (o Backup > Ripristina file…): "Ripristina file" mostra un unico albero di tutti i file e le cartelle mai salvati dai backup; i file che non sono più nell'origine sono segnati { $deleted-icon } eliminato e si possono comunque ripristinare. Spunta file o cartelle nell'albero o nei risultati della ricerca (la casella di ricerca trova un file o una cartella per nome, con maiuscole o minuscole).
doc-restore-versions = Si spunta solo con le caselle. Un clic sul nome di un file elenca a destra le sue versioni, dalla più recente: viene ripristinata quella scelta, la più recente se non ne scegli una. Una cartella spuntata torna per intero, com'era nel backup più recente che la contiene; un clic sul suo nome non fa nulla. Le frecce su e giù passano al file sopra o sotto. Un file dentro una cartella spuntata torna com'era alla data della cartella, a meno che tu non scelga per lui un'altra versione. Una cartella spuntata riporta anche i file che conteneva prima e che da allora sono stati eliminati dall'origine, ciascuno nella sua ultima versione. Ogni file ripristinato che non è più nell'origine prende "_deleted_" davanti al nome (_deleted_relazione.docx).
doc-restore-where = In cima al pannello di destra scegli dove va tutto: accanto all'originale con la data nel nome (relazione.docx torna come "relazione (2026-11-02 21.30).docx" e il file attuale non viene toccato), nella posizione originale o in un'altra cartella; i file già presenti vengono sostituiti solo se spunti "Sostituisci i file già presenti". Ripristina poi riporta tutto in un'unica operazione. "Togli selezione", accanto, toglie tutte le spunte.
doc-verify-title = Controllo di integrità e prova di ripristino
doc-verify-body = Il controllo completo rilegge ogni file salvato di ogni backup e lo confronta con la sua impronta; la prova di ripristino rilegge a caso alcuni file del backup più recente, come farebbe un ripristino, senza scriverli da nessuna parte. Avviali dalla sezione Controlli delle impostazioni del profilo (o da Backup > Controlla i backup ora), oppure falli eseguire ogni tot giorni dopo un backup pianificato. Se emergono problemi, fai un backup completo e controlla il disco di destinazione.
doc-activity-title = Scheda Attività
doc-activity-body = La scheda Attività mostra l'avanzamento del backup, dell'unione, del ripristino o del controllo in corso, con il pulsante Annulla, e il registro di ogni operazione passata, automatica o avviata a mano: che cosa ha fatto e quali file ha saltato perché un altro programma li teneva aperti. Sopra i passi di un'operazione, Apri il registro completo mostra il suo registro intero come file di testo nell'editor: i passi, poi ogni file su cui ha lavorato con l'ora, che cosa gli è successo (nuovo, modificato, salvato come modifiche, copiato di nuovo, eliminato, spostato, versione rimossa, ripristinato, mantenuto, saltato, problema), la dimensione e il percorso completo; i file invariati di un backup incrementale e i file integri di un controllo completo sono solo contati. Copia il registro copia lo stesso testo; Registro del programma apre il registro tecnico di Archeion, per i problemi più profondi.
doc-window-title = Chiusura della finestra
doc-window-body = Chiudere la finestra non ferma Archeion: si sposta nell'area di notifica, accanto all'orologio, e continua a lavorare, quindi i backup vecchi vengono ancora uniti; riapri la finestra dalla sua icona. Archeion inoltre parte con Windows, nascosto lì. File > Esci o Esci dal menu dell'icona lo chiudono davvero. Entrambe le cose si possono disattivare in File > Preferenze, dove scegli anche la lingua. I backup pianificati partono comunque, anche quando Archeion non è in esecuzione.
doc-cli-title = Riga di comando
doc-cli-body = archeion.exe senza argomenti apre la finestra. I comandi qui sotto eseguono un'operazione senza finestra, come fanno le attività pianificate. L'id del profilo è il campo id del profilo in settings.toml.
doc-cli-run = Esegue il backup del profilo (--kind complete per un backup completo).
doc-cli-merge = Unisce i backup vecchi secondo le regole del profilo.
doc-cli-verify = Controlla i backup (--sample N: prova di ripristino su N file).
doc-cli-delete = Elimina un backup.
doc-cli-restore = Ripristina i file elencati in un file di richiesta scritto dalla finestra.
doc-cli-watch = Avvisa dei profili senza un backup recente; l'Utilità di pianificazione lo esegue ogni giorno.
doc-cli-extract = Ripristina da una cartella di backup da sola, senza impostazioni né profilo: per esempio con Archeion portatile su un altro computer. I file vanno sotto la cartella di arrivo con il loro percorso completo.
doc-cli-extract-options = Opzioni di extract: --archive <nome> sceglie il backup (il più recente se manca); --path <file o cartella> ripristina solo quello, indicato con il suo percorso originale; --overwrite sostituisce i file esistenti. Per i backup cifrati indica la password o la chiave di recupero con --stdin (prima riga dello standard input), con la variabile ARCHEION_PASSWORD o ARCHEION_RECOVERY_KEY. L'avanzamento è scritto anche in archeion-extract.log nella cartella di arrivo. Nel Prompt dei comandi avvialo come start /wait archeion extract … perché il prompt lo aspetti.
doc-cli-exit = Codici di uscita: 0 riuscito, 1 non riuscito, 2 destinazione non trovata, 3 occupato (un'altra operazione sta usando la destinazione), 4 password mancante o errata, 5 spazio libero insufficiente.
doc-data-title = Dove sono i dati e i log
doc-data-body = Le impostazioni sono nella cartella %APPDATA%\Archeion\config (settings.toml) e il log di ogni operazione è nella sua sottocartella logs. Una copia portatile (con un file portable.txt accanto ad archeion.exe) tiene invece tutto in una cartella data accanto all'eseguibile. Aiuto > Informazioni su Archeion apre queste cartelle.
doc-data-backups = I backup veri e propri sono solo nella destinazione: una cartella per backup, con la data e l'ora nel nome, che contiene i file salvati e l'elenco di tutti i file in quel momento.

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
    [one] 1 file trovato
   *[other] { $count } file trovati
}
step-folders = { $count ->
    [one] 1 cartella letta
   *[other] { $count } cartelle lette
}
step-new = { $count ->
    [one] 1 nuovo
   *[other] { $count } nuovi
}
step-changed = { $count ->
    [one] 1 modificato
   *[other] { $count } modificati
}
step-unchanged = { $count ->
    [one] 1 invariato
   *[other] { $count } invariati
}
step-copied-again = { $count ->
    [one] 1 invariato ricopiato
   *[other] { $count } invariati ricopiati
}
step-moved = { $count ->
    [one] 1 file spostato
   *[other] { $count } file spostati
}
step-restored = { $count ->
    [one] 1 ripristinato
   *[other] { $count } ripristinati
}
step-kept = { $count ->
    [one] 1 già presente, mantenuto
   *[other] { $count } già presenti, mantenuti
}
step-checked = { $count ->
    [one] 1 controllato
   *[other] { $count } controllati
}
step-damaged = { $count ->
    [one] 1 problema
   *[other] { $count } problemi
}
step-skipped = { $count ->
    [one] 1 saltato
   *[other] { $count } saltati
}
merge-group = { $count } backup, dal { $first } al { $last }, in uno
merge-group-of = Gruppo { $index } di { $of }: { $count } backup, dal { $first } al { $last }, in uno
recent-title = Ultimi file
recent-new = Nuovo
recent-changed = Modificato
recent-copied-again = Ricopiato
recent-moved = Spostato
recent-restored = Ripristinato
recent-kept = Mantenuto
recent-checked = OK
recent-damaged = Problema
recent-skipped = Saltato
recent-deleted = Eliminato
recent-dropped = Versione rimossa

# Profile files (File menu)
import-profile = Importa profilo…
export-profile = Esporta profilo…
import-profile-hint = Aggiunge un profilo da un file esportato da Archeion: le sue regole, non la password.
export-profile-hint = Salva tutte le regole di questo profilo in un file, per conservarlo o portarlo su un altro computer. La password non viene salvata.
profile-file-type = Profilo di Archeion
import-failed-read = Impossibile leggere il file: { $detail }
import-failed-format = { $file } non è un profilo di Archeion.
import-failed-newer = { $file } è stato creato da una versione più recente di Archeion: aggiorna Archeion per importarlo.
export-failed = Impossibile salvare il profilo: { $detail }
import-backup = Importa profilo da un backup…
import-backup-hint = Aggiunge il profilo di una cartella di backup di Archeion, dalla copia che ogni backup vi conserva: per esempio su un computer nuovo. I backup restano dove sono.
import-backup-busy = È in corso l'importazione di un profilo da un backup.
import-backup-password-title = Backup cifrati
import-backup-password-body = I backup in questa cartella sono cifrati: scrivi la loro password per leggere il profilo. Viene salvata con il profilo, come per ogni profilo cifrato.
import-backup-import = Importa
import-backup-none = { $folder } non contiene backup di Archeion: scegli la cartella che contiene archeion.json.
import-backup-no-copy = I backup in { $folder } sono stati fatti da una versione precedente di Archeion e non contengono una copia del profilo. Per usarli, crea un nuovo profilo con questa cartella come destinazione e scegli "Usa questi backup per questo profilo".
import-backup-damaged = La copia del profilo in { $folder } non si legge: { $detail }
import-backup-failed = Impossibile leggere { $folder }: { $detail }
import-backup-password-not-saved = Il profilo è stato importato, ma non è stato possibile salvarne la password: scrivila nelle sue impostazioni. { $detail }
doc-profiles-files = File > Esporta profilo… salva tutte le regole del profilo selezionato (cartelle, esclusioni, destinazione, formato, pianificazione, unione, controlli, notifiche) in un file .archeion-profile, per conservarlo o portarlo su un altro computer; File > Importa profilo… lo aggiunge di nuovo. La password non è mai nel file: un profilo cifrato la chiede dopo l'importazione. Su un altro computer il profilo importato ritrova i suoi backup nella destinazione; importato due volte sullo stesso, diventa una copia con "(2)" nel nome.
doc-profiles-from-backup = Ogni backup conserva anche una copia del suo profilo nella cartella di destinazione (archeion.profile.toml, oppure archeion.profile.zip sigillato con la chiave della destinazione quando è cifrata). Su un computer nuovo, o se le impostazioni sono andate perse, File > Importa profilo da un backup… la legge dalla cartella che scegli: il profilo torna con tutte le sue regole, la destinazione diventa quella cartella (anche se la lettera dell'unità è cambiata) e la pianificazione aspetta che tu la riattivi. I backup cifrati chiedono prima la password. La copia non viene mai usata da sola: la legge solo questo comando.
