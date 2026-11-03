# Menus
menu-file = Fichier
menu-backup = Sauvegarde
menu-help = Aide
new-profile = Nouveau profil
quit = Quitter
restore = Restaurer des fichiers…
data-folder = Dossier des données

# Profile list and empty state
profiles = Profils
unnamed = (sans nom)
new-profile-name = Nouvelle sauvegarde
empty-title = Aucune sauvegarde pour l'instant
empty-body = Créez un profil : choisissez les dossiers à sauvegarder, où les enregistrer et quand.
tab-settings = Paramètres
tab-backups = Sauvegardes

# Profile form
profile-name = Nom
folders = Dossiers à sauvegarder
no-folders = Aucun dossier pour l'instant : ajoutez-en au moins un.
add-folder = Ajouter un dossier…
remove-folder = Retirer ce dossier de la sauvegarde
exclusions = Exclusions
exclusions-hint = Un motif porte sur le chemin complet, sans tenir compte de la casse : * n'importe quel texte dans un nom, ** n'importe quels dossiers, ? un caractère. Par exemple **/node_modules, **/cache/**, **/~$*.
destination = Destination
dest-local = Dossier ou disque externe
dest-network = Partage réseau
dest-hint = Utilisez un dossier réservé à ce profil. Sans le chiffrement (ci-dessous), toute personne qui peut l'ouvrir peut lire les sauvegardes.
dest-free = Espace libre sur la destination : { $free }
dest-foreign-profile = Ce dossier contient les sauvegardes du profil « { $name } » : choisissez un autre dossier pour ce profil.
dest-foreign-deleted = Ce dossier contient les sauvegardes d'un profil qui n'existe plus.
dest-adopt = Utiliser ces sauvegardes pour ce profil
dest-adopt-hint = Les sauvegardes existantes deviennent celles de ce profil : la prochaine sauvegarde les prolonge, et elles peuvent être restaurées et fusionnées d'ici.
dest-adopt-failed = Impossible de reprendre les sauvegardes : { $detail }
dest-foreign-hint = La destination contient les sauvegardes d'un autre profil : voir la section Destination.
run-foreign-destination = La destination contient les sauvegardes d'un autre profil. Ouvrez les paramètres du profil : si ce profil a été supprimé, « Utiliser ces sauvegardes pour ce profil » les reprend.
browse = Parcourir…
container = Format
container-zip-per-file = Un zip par fichier
container-folder = Copies simples
container-zip-per-file-hint = Chaque fichier est compressé séparément (rapport.docx devient rapport.docx.zip) dans les mêmes dossiers ; il s'ouvre avec l'Explorateur de fichiers ou 7-Zip.
container-folder-hint = Les fichiers sont copiés tels quels, avec les mêmes noms, sans compression.
large-files = Enregistrer les gros fichiers en tant que modifications seulement, à partir de
large-files-unit = Mo
large-files-hint = Les fichiers d'au moins cette taille sont enregistrés par morceaux : après une modification, une sauvegarde n'enregistre que les morceaux modifiés. Les sauvegardes complètes les conservent aussi entiers, ils s'ouvrent donc toujours avec 7-Zip. Les fichiers plus petits sont enregistrés comme ci-dessus.
large-files-change-hint = Vous pouvez le modifier à tout moment : les sauvegardes suivantes suivent le nouveau seuil, celles déjà faites restent telles quelles.
large-files-encrypted = Chaque fichier est enregistré par morceaux : d'un fichier modifié, seul ce qui a changé est enregistré ; les sauvegardes complètes conservent aussi des copies entières.
schedule = Sauvegarde automatique
schedule-manual = Seulement quand je la lance
schedule-daily = Tous les jours
schedule-weekly = Toutes les semaines
at = à
schedule-hint = Si l'ordinateur est éteint à cette heure-là, elle s'exécute dès qu'il est rallumé.
merge = Fusion des anciennes sauvegardes
merge-hint = Les anciennes sauvegardes ne sont pas supprimées mais fusionnées avec la suivante conservée : chaque date conservée reste complète et de l'espace est libéré. La première sauvegarde complète n'est jamais fusionnée automatiquement.
merge-automatic-hint = La fusion est automatique : tant qu'Archeion est ouvert, il fusionne les anciennes sauvegardes selon les règles ci-dessous, jamais pendant qu'une sauvegarde ou une autre opération les utilise.
retention-hint = Une sauvegarde est conservée si au moins une règle la retient. Les sauvegardes des jours indiqués dans « Toutes, pendant jours » ne sont jamais fusionnées.
keep-within = Toutes, pendant jours
keep-last = Les plus récentes
keep-daily = Une par jour, pendant jours
keep-weekly = Une par semaine, pendant semaines
keep-monthly = Une par mois, pendant mois
keep-yearly = Une par an, pendant années
keep-everything = Toutes les règles à 0 : chaque sauvegarde est conservée et rien n'est fusionné.
back-up-now = Sauvegarder maintenant
merge-now = Fusionner maintenant
merge-now-hint = Applique les règles de fusion maintenant ; la première sauvegarde complète reste telle quelle.
busy-hint = Une sauvegarde ou une fusion est déjà en cours.
fix-profile-hint = Complétez d'abord le profil.
delete-profile = Supprimer le profil
delete-profile-hint = Demande confirmation, puis supprime le profil et ses tâches planifiées ; les sauvegardes restent sur la destination.
delete-profile-title = Supprimer le profil « { $name } » ?
delete-profile-body = Le profil et ses tâches planifiées sont supprimés. Tant que vous ne fermez pas le message dans la barre d'état, Annuler les rétablit.
delete-profile-password = Le mot de passe enregistré est supprimé du Gestionnaire d'identification Windows quand l'annulation n'est plus possible : quand vous fermez ce message, supprimez un autre profil ou quittez Archeion.
delete-profile-backups = Les sauvegardes dans { $path } ne sont pas supprimées : un nouveau profil qui utilise ce dossier peut les reprendre avec « Utiliser ces sauvegardes pour ce profil ».

day-monday = lundi
day-tuesday = mardi
day-wednesday = mercredi
day-thursday = jeudi
day-friday = vendredi
day-saturday = samedi
day-sunday = dimanche

error-empty-name = Donnez un nom au profil.
error-no-sources = Ajoutez au moins un dossier à sauvegarder.
error-no-destination = Choisissez où enregistrer les sauvegardes.
error-destination-in-source = La destination se trouve dans un dossier sauvegardé ; choisissez-en une autre.

# Backups tab
refresh = Actualiser
reading-backups = Lecture des sauvegardes…
no-backups-yet = Aucune sauvegarde pour ce profil pour l'instant.
kind-complete = Complète
kind-incremental = Incrémentielle

# Status bar
ready = Prêt
cancel = Annuler
undo = Annuler
phase-starting = Démarrage…
phase-opening = Ouverture de la destination…
phase-scanning = Recherche des modifications…
phase-copying = Copie…
phase-merging = Fusion des anciennes sauvegardes…
phase-restoring = Restauration…
phase-verifying = Vérification…
progress-of = { $done } sur { $total }
skipped-count = { $count ->
    [one] { $count } fichier ignoré (en cours d'utilisation)
   *[other] { $count } fichiers ignorés (en cours d'utilisation)
}
profile-removed = Profil « { $name } » supprimé.
run-finished = « { $name } » sauvegardé : { $new } nouveaux, { $changed } modifiés, { $deleted } supprimés, { $added } copiés.
merge-finished = « { $name } » fusionné : de { $before } à { $after } sauvegardes, { $freed } libérés.
restore-finished = { $count } fichiers restaurés.
verify-finished = { $count } fichiers vérifiés, { $problems } problèmes.
run-destination-missing = Destination introuvable : { $location }. Branchez le disque et réessayez.
run-busy = Une autre sauvegarde ou fusion de ce profil est en cours ; réessayez quand elle sera terminée.
run-failed = L'opération a échoué : { $detail }
run-no-space = Espace libre insuffisant sur la destination : { $needed } nécessaires, { $available } libres. Libérez de l'espace, ou fusionnez ou supprimez d'anciennes sauvegardes, puis relancez : les fichiers déjà copiés sont conservés et ne sont pas recopiés.
restore-no-space = Espace libre insuffisant là où les fichiers sont restaurés : { $needed } nécessaires, { $available } libres. Rien n'a été écrit : libérez de l'espace, ou restaurez moins de fichiers ou sur un autre disque.
backup-resumed = { $count ->
    [one] { $count } fichier repris de la sauvegarde interrompue
   *[other] { $count } fichiers repris de la sauvegarde interrompue
}
backup-as-changes = { $count ->
    [one] { $count } fichier enregistré en tant que modifications seulement
   *[other] { $count } fichiers enregistrés en tant que modifications seulement
}
run-cancelled = Annulé. Rien n'a été modifié dans les sauvegardes ; une sauvegarde arrêtée reprendra là où elle s'est arrêtée la prochaine fois.

# Errors
settings-unreadable = Le fichier des paramètres est illisible, les modifications ne seront donc pas enregistrées : { $detail }
save-failed = Impossible d'enregistrer les paramètres : { $detail }
schedule-failed = Impossible de planifier l'exécution automatique : { $detail }
start-failed = Impossible de démarrer l'opération : { $detail }
unexpected-stop = L'opération s'est arrêtée de façon inattendue.

# Restore browser
dates = Dates des sauvegardes
archive-tooltip = { $files } fichiers, { $stored } enregistrés dans cette sauvegarde ({ $size })
search-hint = Rechercher un fichier par nom dans toutes les sauvegardes…
no-results = Aucun fichier portant ce nom.
results = { $count ->
    [one] { $count } fichier trouvé
   *[other] { $count } fichiers trouvés
}
close = Fermer
restore-to = Restaurer vers :
target-beside = À côté de l'original, avec la date
target-beside-hint = rapport.docx est restauré sous le nom « rapport (2026-11-02 21.30).docx » : le fichier actuel n'est pas modifié.
target-original = Emplacement d'origine
target-folder = Un autre dossier
replace-existing = Remplacer les fichiers qui existent déjà
replace-warning = Les fichiers actuels seront remplacés par la copie de la sauvegarde.
tick-files-hint = Cochez au moins un fichier ou un dossier.
choose-folder-hint = Choisissez le dossier de destination de la restauration.
restore-selected = Restaurer
clear-selection = Effacer la sélection
tick-folder = Restaurer ce dossier et tout son contenu
tick-file = Restaurer ce fichier
backups-restore-files = Restaurer des fichiers
backups-restore-files-hint = Tous les fichiers jamais sauvegardés : cochez ce qu'il faut récupérer, choisissez les versions ou la date, et la destination.
backups-manage = Gérer les sauvegardes
backups-manage-hint = La liste des dates de sauvegarde : fusionnez-en plusieurs en une seule, ou supprimez-en une.
restore-tree-count = { $files ->
        [one] { $files } fichier
       *[other] { $files } fichiers
    } dans les sauvegardes · supprimés à la source : { $deleted }
restore-tree-hint = Cochez des fichiers ou des dossiers pour les restaurer ; la recherche trouve un fichier par son nom. Les fichiers marqués { $deleted-icon } supprimé ne sont plus à la source et peuvent encore être restaurés.
deleted-label = supprimé
tree-deleted-file = Absent de la sauvegarde la plus récente : supprimé à la source. Cochez-le pour restaurer l'une de ses versions.
tree-deleted-folder = Absent de la sauvegarde la plus récente : supprimé à la source. Cochez-le pour le restaurer tel qu'il était à une date.
tick-in-folder = Restauré avec le dossier coché { $folder }. Décochez-le pour l'exclure.
tick-not-in-folder-date = Absent de la sauvegarde avec laquelle revient le dossier coché { $folder } : il n'est pas restauré. Décochez ce dossier pour le choisir seul.
tick-partial = Un élément à l'intérieur est coché, ou exclu du dossier coché. Cliquez pour cocher tout le dossier.
tick-full = Tout son contenu est coché élément par élément. Cliquez pour tout décocher.
tick-left-out = Exclu du dossier coché { $folder } : il n'est pas restauré. Cochez-le pour le restaurer avec le dossier.
selection-empty = Rien à restaurer pour l’instant. Cochez des fichiers ou des dossiers dans l’arborescence.
restore-to-hint = Où va tout ce qui est coché.
restore-conflict = Deux éléments cochés remplaceraient le même fichier d’origine : décochez l’un des deux.
restore-conflict-hint = Deux éléments cochés remplaceraient le même fichier d’origine : décochez l’un des deux.
restore-wait-versions = Lecture des versions des fichiers cochés…
restore-summary = { $files ->
        [one] { $files } fichier
       *[other] { $files } fichiers
    }, { $folders ->
        [one] { $folders } dossier
       *[other] { $folders } dossiers
    } — { $size }

# Manual merge
merge-choose = Choisir les sauvegardes à fusionner…
merge-choose-hint = Cochez la première et la dernière sauvegarde : tout ce qui se trouve entre les deux devient une seule sauvegarde.
merge-tick = Cocher comme extrémité de la plage à fusionner
merge-tick-hint = Cochez la sauvegarde la plus ancienne et la plus récente à fusionner ; celles qui se trouvent entre les deux sont incluses.
merge-summary = { $count } sauvegardes deviennent une seule, datée du { $date }.
merge-complete-warning = La plage comprend la première sauvegarde complète : elle est remplacée par une nouvelle sauvegarde complète datée de la plus récente choisie, et les dates antérieures ne pourront plus être restaurées.
merge-complete-ack = J'ai compris
merge-two-hint = Cochez au moins deux sauvegardes.
merge-ack-hint = Confirmez que vous avez compris l'avertissement.
merge-selected = Fusionner

# Activity tab
tab-activity = Activité
show-details = Détails
cancel-hint = Arrêter maintenant : les sauvegardes restent comme avant cette exécution. La prochaine sauvegarde réutilise les fichiers déjà copiés.
activity-backup-running = Sauvegarde de « { $name } »
activity-merge-running = Fusion des sauvegardes de « { $name } »
activity-restore-running = Restauration depuis « { $name } »
activity-preparing = Préparation…
activity-file = Fichier
activity-folder = Dossier
activity-files = Fichiers
activity-speed = Vitesse
activity-elapsed = Écoulé
activity-remaining = Restant
count-of = { $done } sur { $total }
files-found = { $count } trouvés jusqu'ici
about = environ { $time }
activity-took = Durée : { $time }.
activity-backup-summary = Terminé : { $new } nouveaux, { $changed } modifiés, { $deleted } supprimés, { $unchanged } inchangés ; { $copied } copiés.
activity-merge-summary = Terminé : de { $before } à { $after } sauvegardes, { $freed } libérés.
activity-restore-summary = Terminé : { $restored } fichiers restaurés, { $kept } déjà présents et conservés.
skipped-hint = Ces fichiers n'ont pas pu être lus, en général parce qu'un autre programme les a ouverts. Leur version précédente, s'il y en a une, est toujours dans les sauvegardes.
skipped-more = …et { $count } de plus (voir le journal).

# Exclusions and format lock
no-exclusions = Rien n'est exclu : tous les fichiers des dossiers sont sauvegardés.
remove-exclusion = Sauvegarder à nouveau
exclusion-folder = Dossier et tout son contenu
exclusion-file = Fichier
exclusion-type = Tous les fichiers de ce type
exclusion-pattern = Motif
exclude-folder = Exclure un dossier…
exclude-file = Exclure un fichier…
exclude-type-hint = .tmp
exclude-type = Exclure le type
common-types = Courants :
advanced = Avancé
exclude-pattern = Exclure des motifs
exclude-pattern-add = Ajouter
container-locked = La destination contient déjà des sauvegardes dans ce format : il ne peut pas changer. Pour utiliser l'autre, choisissez une destination vide.
container-unreachable = La destination est inaccessible, le format ne peut donc pas être changé maintenant.
container-checking = Vérification de la destination…
activity-idle-title = Aucune opération en cours
activity-idle-body = Les sauvegardes, fusions et restaurations affichent leur progression ici.

# Backup kinds and deletion
menu-run-incremental = Sauvegarde incrémentielle maintenant
menu-run-complete = Sauvegarde complète maintenant
run-now = Lancer maintenant :
run-incremental = Incrémentielle
run-incremental-hint = Copie seulement les fichiers nouveaux et modifiés ; rapide, c'est le choix habituel.
run-complete = Complète
run-complete-hint = Copie tous les fichiers et commence une nouvelle chaîne : les sauvegardes suivantes ne dépendent plus des précédentes. Prend plus de temps et plus d'espace.
run-merge = Fusionner
run-first = Lancer la première sauvegarde
run-incremental-button = Lancer une sauvegarde incrémentielle
run-complete-button = Lancer une sauvegarde complète
full-backup = Sauvegarde complète :
full-never = Seulement la première
full-every-backups = Toutes les
full-every-days = Quand la dernière date de plus de
full-backups-unit = sauvegardes
full-days-unit = jours
full-backup-hint = S'applique aux sauvegardes automatiques ; une sauvegarde complète commence une nouvelle chaîne, ce qui rend la restauration indépendante des sauvegardes plus anciennes.
run-finished-complete = « { $name } » sauvegardé entièrement : { $new } nouveaux, { $changed } modifiés, { $deleted } supprimés, { $added } copiés.
activity-complete-summary = Sauvegarde complète terminée : { $new } nouveaux, { $changed } modifiés, { $deleted } supprimés, { $unchanged } inchangés ; { $copied } copiés.
activity-delete-running = Suppression d'une sauvegarde de « { $name } »
activity-delete-summary = Terminé : de { $before } à { $after } sauvegardes, { $freed } libérés.
delete-finished = Une sauvegarde de « { $name } » a été supprimée.
delete-backup = Supprimer cette sauvegarde…
delete-backup-hint = Supprime la date sélectionnée ; les sauvegardes suivantes restent complètes.
delete-backup-confirm = Supprimer la sauvegarde
delete-initial-hint = La première sauvegarde complète ne peut être supprimée que si elle est la seule : les autres en dépendent.
delete-complete-hint = Une sauvegarde complète commence une chaîne dont dépendent les sauvegardes suivantes : elle ne peut pas être supprimée. Fusionnez-la plutôt avec les suivantes.
delete-only-warning = La sauvegarde du { $date } est la seule : la destination sera vide et la prochaine sauvegarde sera complète. Cette action est irréversible.
delete-newest-warning = La sauvegarde du { $date } sera supprimée, avec les versions de fichiers enregistrées uniquement dans celle-ci. Cette action est irréversible.
delete-middle-warning = La sauvegarde du { $date } sera supprimée. Les fichiers dont les sauvegardes suivantes ont besoin passent dans la sauvegarde suivante ; les versions enregistrées uniquement dans celle-ci sont perdues. Cette action est irréversible.

# Encryption
encrypt = Chiffrer les sauvegardes (AES-256, noms de fichiers compris)
encrypt-hint = Chaque fichier est un zip AES-256 au nom masqué ; sans le mot de passe, personne ne peut lire le contenu ni voir quels fichiers sont enregistrés. Le mot de passe est conservé dans le Gestionnaire d'identification Windows.
encrypt-needs-zip = Le chiffrement nécessite le format « Un zip par fichier ».
password = Mot de passe
password-again = Répétez le mot de passe
password-set = Choisissez le mot de passe
password-reenter = Saisissez le mot de passe de ces sauvegardes
password-saved = Mot de passe enregistré dans le Gestionnaire d'identification Windows.
password-missing = Aucun mot de passe enregistré sur cet ordinateur : saisissez-le pour lancer des sauvegardes et les parcourir.
password-loss-warning = Sans ce mot de passe, ou la clé de récupération affichée après la première sauvegarde, les sauvegardes ne peuvent pas être ouvertes et personne ne peut les récupérer. Conservez le mot de passe en lieu sûr.
password-save = Enregistrer le mot de passe
password-checking = Vérification du mot de passe…
password-empty-hint = Saisissez le mot de passe.
password-short-hint = Utilisez au moins { $count } caractères.
password-mismatch-hint = Les deux mots de passe sont différents.
password-rule-length = Au moins { $count } caractères (actuellement { $typed })
password-rule-match = Les deux mots de passe correspondent
password-rule-variety = Au moins { $kinds } types de caractères parmi minuscules, majuscules, chiffres et symboles (inutile à partir de { $passphrase } caractères)
password-rule-common = Pas un mot de passe courant, comme « Password2024 » ou « qwerty »
password-rule-runs = Pas de suite de { $count } caractères répétés ou consécutifs, comme « aaaa », « 1234 » ou « abcd »
password-rule-name = Ne contient pas le nom du profil
password-rule-strength = Assez difficile à deviner : robustesse au moins { $level }
password-rule-tip = Tous les caractères sont acceptés, espaces compris : une phrase de quatre ou cinq mots est facile à retenir et difficile à deviner, et à partir de { $passphrase } caractères, elle n'a besoin ni de chiffres, ni de majuscules, ni de symboles.
password-strength = Robustesse : { $level }
password-strength-weak = Faible
password-strength-fair = Moyenne
password-strength-strong = Forte
password-strength-very-strong = Très forte
password-variety-hint = Mélangez minuscules, majuscules, chiffres et symboles, ou écrivez une phrase d'au moins { $passphrase } caractères.
password-common-hint = C'est l'un des mots de passe les plus utilisés : choisissez-en un autre.
password-runs-hint = Évitez les suites comme « aaaa » ou « 1234 ».
password-name-hint = Ne mettez pas le nom du profil dans le mot de passe.
password-weak-hint = Trop facile à deviner : allongez-le, par exemple avec quelques mots de plus.
password-first-hint = Saisissez ci-dessus le mot de passe des sauvegardes chiffrées (deux fois pour de nouvelles sauvegardes, et cochez « J'ai compris ») : le bouton l'enregistre et lance la sauvegarde.
password-wrong = Ce mot de passe n'ouvre pas les sauvegardes de la destination.
password-unreachable = Le mot de passe n'a pas pu être vérifié
run-password-needed = Les sauvegardes sont chiffrées et aucun mot de passe n'est enregistré sur cet ordinateur : saisissez-le dans les paramètres du profil.
run-wrong-password = Le mot de passe enregistré n'ouvre pas les sauvegardes : saisissez-le à nouveau dans les paramètres du profil.

# Operations log
log-runs = Opérations
log-only-profile = Ce profil seulement
log-running = En cours
log-empty = Aucune opération pour l'instant.
log-pick = Choisissez une opération à gauche.
log-title = Journal
log-no-lines = Rien n'a encore été journalisé.
log-scheduled = automatique
log-manual = lancée manuellement
log-started = Lancée { $time }
log-backup = sauvegarde { $name }
log-unfinished = Non terminée : interrompue, ou toujours en cours.
log-skipped = Ignoré { $path } : { $reason }
log-op-backup = Sauvegarde
log-op-incremental = Sauvegarde incrémentielle
log-op-complete = Sauvegarde complète
log-op-merge = Fusion
log-op-delete = Suppression de sauvegarde
log-op-restore = Restauration
log-open-full = Ouvrir le journal complet
log-open-full-hint = Écrit le journal complet de cette opération dans un fichier texte et l'ouvre dans votre éditeur de texte : chaque étape, puis chaque fichier traité et ce qui en a été fait, y compris les fichiers ignorés.
log-copy = Copier le journal
log-copy-hint = Copie le journal complet de cette opération sous forme de texte, pour le coller dans un e-mail ou un signalement de problème.
log-copied = Journal copié.
log-open-program = Journal du programme
log-open-program-hint = Ouvre archeion.log, le journal technique du programme : utile pour examiner un problème ou le signaler.
log-open-folder = Dossier des journaux
log-open-folder-hint = Ouvre le dossier des journaux des opérations dans l'Explorateur de fichiers.
log-program-missing = Le journal du programme n'existe pas encore.
log-unreadable = Le fichier journal est illisible.
log-report-failed = Impossible d'afficher le journal complet : { $detail }
log-level-info = Info
log-level-done = OK
log-level-warning = Avertissement
log-level-error = Erreur
log-preparing = Préparation du journal complet…
log-files-title = Fichiers
log-files-missing = La liste des fichiers n'a pas été enregistrée pour cette opération : elle a été faite par une version antérieure d'Archeion.
log-files-none = Aucun fichier traité.
log-file-new-chunks = enregistré sous forme de modifications : { $size } de nouveaux blocs écrits
log-file-resumed = déjà copié par la sauvegarde interrompue, conservé
log-count-new = Nouveaux : { $count }
log-count-changed = Modifiés : { $count }
log-count-as-changes = Enregistrés sous forme de modifications : { $count }
log-count-copied-again = Copiés à nouveau : { $count }
log-count-unchanged = Inchangés, non listés : { $count }
log-count-deleted = Supprimés : { $count }
log-count-moved = Déplacés : { $count }
log-count-dropped = Versions retirées : { $count }
log-count-restored = Restaurés : { $count }
log-count-kept = Conservés : { $count }
log-count-checked = Relus intacts : { $count }
log-count-checked-unlisted = Relus intacts, non listés : { $count }
log-count-damaged = Problèmes : { $count }
log-count-skipped = Ignorés : { $count }

# Password change and recovery key
password-change = Changer le mot de passe
password-new = Nouveau mot de passe
password-change-button = Changer le mot de passe
recovery-later = La clé de récupération sera disponible après la première sauvegarde.
recovery-show = Afficher la clé de récupération…
recovery-title = Clé de récupération
recovery-hint = Avec cette clé, 7-Zip ouvre chaque fichier et manifeste chiffré, même sans Archeion et sans le mot de passe. Le manifeste de chaque sauvegarde indique où chaque fichier est stocké.
recovery-warning = Quiconque possède cette clé peut lire toutes les sauvegardes. Imprimez-la ou notez-la et conservez-la hors ligne ; elle ne change pas quand vous changez le mot de passe.
recovery-copy = Copier
recovery-copy-hint = Copie la clé sans espaces, comme l'attend 7-Zip.
recovery-hide = Masquer
password-current = Mot de passe actuel
password-current-hint = Saisissez le mot de passe actuel.
recovery-ask = Pour afficher la clé de récupération, saisissez le mot de passe actuel de ces sauvegardes.
recovery-show-button = Afficher

# Integrity check
verify-section = Vérifications
verify-hint = La vérification complète relit chaque fichier stocké de chaque sauvegarde et le compare à son empreinte ; le test de restauration relit au hasard quelques fichiers de la sauvegarde la plus récente. Les vérifications dues s'exécutent après une sauvegarde planifiée.
verify-full-every = Vérification complète tous les
verify-sample-every = Test de restauration tous les
verify-sample-files = sur fichiers :
verify-zero-hint = 0 jour = jamais automatiquement.
verify-full-now = Vérifier les sauvegardes maintenant
verify-full-hint = Relit chaque fichier stocké et le vérifie ; sur une grande destination, cela prend autant de temps que la lecture de toutes les sauvegardes.
verify-sample-now = Test de restauration maintenant
verify-sample-hint = Relit au hasard quelques fichiers de la sauvegarde la plus récente, comme le ferait une restauration, sans les écrire nulle part.
verify-no-backups = Il n'y a encore aucune sauvegarde à vérifier.
verify-problems = Problèmes détectés
verify-problems-hint = Les sauvegardes indiquées risquent de ne pas se restaurer correctement. Faites une sauvegarde complète maintenant et vérifiez le disque de destination.
activity-verify-running = Vérification des sauvegardes de « { $name } »
log-op-verify = Contrôle d'intégrité
log-op-restore-test = Test de restauration

# Automatic backup: schedules, events, notifications
schedule-days = Les jours choisis
schedule-hours = Toutes les quelques heures
schedule-monthly = Une fois par mois
schedule-all-days = Tous les jours
schedule-add-time = + Ajouter une heure
schedule-remove-time = Supprimer cette heure
schedule-every = Toutes les
hours-unit = heures
schedule-from = de
schedule-until = à
schedule-hours-hint = La dernière exécution commence avant l'heure de fin ; une heure de début égale à l'heure de fin signifie 24 h sur 24.
schedule-month-day = Le
schedule-month-hint = Jours 1 à 28 uniquement, pour que chaque mois ait ce jour.
events-title = Sauvegarder aussi :
event-logon = Quelques minutes après ma connexion à Windows
event-idle = Quand l'ordinateur est inactif depuis 10 minutes
event-drive = Quand un lecteur est connecté
event-drive-hint = Utile pour un disque externe comme destination : la sauvegarde démarre quand il est branché, et rien ne se passe quand un autre lecteur l'est.
event-min-hours = Pas plus souvent que toutes les
event-hint = Une sauvegarde déclenchée par l'un de ces événements est ignorée si la dernière sauvegarde est plus récente, ou si la destination n'est pas connectée.
alerts = Notifications
alert-on-failure = M'avertir quand une sauvegarde automatique échoue
alert-stale = M'avertir quand il n'y a pas eu de sauvegarde depuis
alert-hint = Notifications Windows ; 0 jour = aucun avertissement. La vérification a lieu chaque jour et après la connexion.
alert-manual-hint = Uniquement pour les sauvegardes automatiques : choisissez une planification dans « Sauvegarde automatique ».
alert-failed-title = Archeion : échec de la sauvegarde de « { $name } »
alert-stale-title = Archeion : « { $name } » non sauvegardé
alert-stale-body = Aucune sauvegarde depuis { $days } jours (dernière le { $when }). Vérifiez que la destination est connectée, ou lancez une sauvegarde depuis Archeion.
closing-after-run = Une opération est en cours : Archeion se fermera tout seul à la fin.
closing-cancel = Ne pas fermer

# Notification area and Preferences window
menu-preferences = Préférences…
preferences-title = Préférences
preferences-window = Fenêtre
language = Langue
language-system = Langue du système
language-tray-restart = Le menu de l'icône de la zone de notification changera de langue au prochain démarrage d'Archeion.
quit-hint = Ferme Archeion ; les sauvegardes planifiées s'exécutent quand même depuis le Planificateur de tâches Windows.
tray-open = Ouvrir Archeion
tray-quit = Quitter
tray-running = opération en cours
option-tray = Rester dans la zone de notification quand la fenêtre est fermée
option-tray-hint = Fermer la fenêtre la masque à côté de l'horloge : Archeion continue de fusionner les anciennes sauvegardes et de terminer l'opération en cours. Cliquez sur son icône pour la rouvrir, ou choisissez Quitter.
option-tray-unavailable = Windows n'a pas fourni d'icône dans la zone de notification.
option-autostart = Démarrer Archeion avec Windows
option-autostart-hint = À l'ouverture de session, Archeion démarre masqué dans la zone de notification. Vous pouvez aussi désactiver ce démarrage dans le Gestionnaire des tâches, sous Applications de démarrage.
autostart-failed = Impossible de modifier le démarrage avec Windows : { $detail }

# Help menu: user guide and About
menu-documentation = Documentation
menu-about = À propos d'Archeion
about-version = Version { $version }
about-name = Le nom vient du grec ancien ἀρχεῖον, « archive ».
about-author = Auteur : MR
about-license = Licence : GPL-3.0
about-repository = Code source sur GitHub
about-logs-folder = Dossier des journaux
about-open-folder = Ouvrir le dossier
about-folder-missing = Le dossier n'existe pas encore.
doc-title = Documentation d'Archeion
doc-intro = Archeion conserve des copies de vos dossiers sur un autre disque, un partage réseau ou tout autre dossier, et vous rend n'importe quel fichier tel qu'il était à une date passée. Ouvrez une section pour la lire.
doc-what-title = Ce que fait Archeion
doc-what-backups = La première sauvegarde d'un profil est complète : elle copie tous les fichiers. Les suivantes sont incrémentielles : elles ne copient que les fichiers nouveaux et modifiés, elles sont donc rapides et légères, et pourtant chaque date peut être restaurée en entier. Sous « Sauvegarde complète », dans les paramètres du profil, vous pouvez aussi demander une nouvelle sauvegarde complète toutes les quelques sauvegardes, ou quand la dernière date de plus d'un certain nombre de jours ; une sauvegarde complète commence une nouvelle chaîne qui ne dépend plus des sauvegardes plus anciennes.
doc-what-storage = Chaque fichier est stocké séparément, dans les mêmes dossiers que l'original : sous forme de zip de ce fichier (« Un zip par fichier » : rapport.docx devient rapport.docx.zip), de copie simple ou de zip chiffré en AES-256. Les sauvegardes sont lisibles sans Archeion : l'Explorateur de fichiers et 7-Zip ouvrent les zip, et 7-Zip ouvre les zip chiffrés avec la clé de récupération.
doc-what-safe = Une sauvegarde interrompue (ordinateur éteint, disque retiré, Annuler) laisse les sauvegardes existantes telles qu'elles étaient ; la sauvegarde suivante reprend là où elle s'est arrêtée et ne recopie pas les fichiers déjà copiés.
doc-profiles-title = Profils
doc-profiles-body = Un profil indique quoi sauvegarder, où et quand. Créez-en un avec Fichier > Nouveau profil (Ctrl+N) et remplissez son onglet Paramètres : un nom, les dossiers à sauvegarder et la destination, un dossier, un disque externe ou un partage réseau. Utilisez chaque dossier de destination pour un seul profil.
doc-profiles-exclusions = Les exclusions laissent de côté ce dont vous n'avez pas besoin : un dossier et tout son contenu, un fichier, ou tous les fichiers d'un type comme .tmp. Avec « Exclure des motifs », vous pouvez ajouter des motifs, comparés au chemin complet sans tenir compte de la casse : * correspond à n'importe quel texte dans un nom, ** à n'importe quels dossiers, ? à un caractère. Par exemple **/node_modules ou **/cache/**.
doc-profiles-format = Le format de stockage (un zip par fichier ou copies simples) et le chiffrement doivent être choisis avant la première sauvegarde : une fois que la destination contient des sauvegardes, ils ne peuvent plus changer. Pour en changer, choisissez une destination vide.
doc-profiles-delete = Supprimer le profil demande confirmation, puis supprime le profil et ses tâches planifiées ; les sauvegardes restent sur la destination. Juste après, Annuler dans la barre d'état rétablit le profil. Un profil qui utilise ce dossier plus tard peut les reprendre avec « Utiliser ces sauvegardes pour ce profil ».
doc-auto-title = Sauvegardes automatiques
doc-auto-schedule = Sous « Sauvegarde automatique », choisissez quand les sauvegardes s'exécutent toutes seules : les jours choisis à une ou plusieurs heures, toutes les quelques heures entre deux heures, ou une fois par mois un jour de 1 à 28. Archeion enregistre la planification dans le Planificateur de tâches Windows, si bien que les sauvegardes s'exécutent même quand Archeion n'est pas ouvert.
doc-auto-missed = Si l'ordinateur est éteint à l'heure prévue, la sauvegarde s'exécute dès qu'il est rallumé.
doc-auto-alerts = Sous « Notifications », Archeion peut afficher une notification Windows quand une sauvegarde automatique échoue, et un avertissement quand un profil n'a pas eu de sauvegarde depuis un certain nombre de jours (vérifié chaque jour et après l'ouverture de session).
doc-auto-now = Vous pouvez toujours lancer une sauvegarde vous-même : Sauvegarder maintenant dans les paramètres du profil, ou Sauvegarde > Sauvegarde incrémentielle maintenant (Ctrl+B) et Sauvegarde complète maintenant.
doc-merge-title = Fusion des anciennes sauvegardes
doc-merge-body = Les anciennes sauvegardes ne sont pas supprimées mais fusionnées avec la sauvegarde suivante conservée : chaque date conservée reste complète, et l'espace des versions devenues inutiles est libéré. La fusion est automatique : elle a lieu tant qu'Archeion est ouvert, jamais pendant qu'une sauvegarde ou une autre opération utilise la destination.
doc-merge-rules = Les règles de « Fusion des anciennes sauvegardes » indiquent quelles sauvegardes conserver, et une sauvegarde est conservée si au moins une règle la retient : toutes les sauvegardes des derniers jours, les plus récentes, une par jour, une par semaine, une par mois et une par an, chacune pour le nombre indiqué. Avec toutes les règles à 0, toutes les sauvegardes sont conservées et rien n'est fusionné.
doc-merge-first = La première sauvegarde complète n'est jamais fusionnée automatiquement.
doc-merge-manual = Dans Sauvegardes > Gérer les sauvegardes, « Choisir les sauvegardes à fusionner… » fusionne manuellement : cochez la sauvegarde la plus ancienne et la plus récente, et tout ce qui se trouve entre les deux devient une seule sauvegarde. Si la plage comprend la première sauvegarde complète, Archeion vous avertit d'abord, car les dates antérieures ne pourront plus être restaurées.
doc-merge-delete = « Supprimer cette sauvegarde… » dans Sauvegardes > Gérer les sauvegardes supprime une date ; les sauvegardes suivantes restent complètes. Une sauvegarde complète dont dépendent des sauvegardes ultérieures ne peut pas être supprimée (fusionnez-la plutôt). Une suppression est irréversible.
doc-encrypt-title = Chiffrement
doc-encrypt-body = Avec « Chiffrer les sauvegardes », chaque fichier devient un zip AES-256 au nom masqué : sans le mot de passe, personne ne peut lire le contenu ni voir quels fichiers sont enregistrés. Le chiffrement nécessite le format « Un zip par fichier » et doit être activé avant la première sauvegarde ; il ne peut plus être modifié ensuite.
doc-encrypt-password = Le mot de passe est enregistré dans le Gestionnaire d'identification Windows, si bien que les sauvegardes automatiques n'exigent aucune saisie. Vous pouvez le changer plus tard dans les paramètres du profil, en saisissant le mot de passe actuel. Sur un autre ordinateur, saisissez-le une fois dans les paramètres du profil.
doc-encrypt-recovery = Après la première sauvegarde, « Afficher la clé de récupération… » affiche une clé avec laquelle 7-Zip ouvre chaque fichier chiffré, même sans Archeion et sans le mot de passe. Imprimez-la ou notez-la et conservez-la hors ligne : sans le mot de passe et sans la clé, personne ne peut ouvrir les sauvegardes. La clé ne change pas quand vous changez le mot de passe. Archeion demande le mot de passe actuel avant de l'afficher.
doc-large-title = Gros fichiers
doc-large-body = Un gros fichier qui change peu (une boîte aux lettres, le disque d'une machine virtuelle, une base de données) n'est pas recopié en entier : il est enregistré en morceaux d'environ 1 Mo, et chaque sauvegarde n'enregistre que les nouveaux morceaux. Les morceaux identiques ne sont enregistrés qu'une fois. Cela vaut pour les fichiers d'au moins la taille choisie dans la section Format du profil (500 Mo sauf modification ; désactivable) et pour tous les fichiers d'une destination chiffrée.
doc-large-complete = Les sauvegardes complètes conservent toujours chaque fichier entier, en zip, en copie simple ou en zip chiffré, et peuvent donc être restaurées avec 7-Zip seul ; seules les sauvegardes incrémentielles qui les suivent contiennent des morceaux. La restauration réassemble les morceaux et vérifie chacun ; la vérification d'intégrité lit tous les morceaux.
doc-large-upgrade = Les sauvegardes faites par les versions précédentes restent telles quelles et toujours lisibles. Les morceaux commencent avec la prochaine sauvegarde complète : sur une telle destination, la prochaine sauvegarde automatique est complète.
doc-restore-title = Restauration des fichiers
doc-restore-body = Ouvrez l'onglet Sauvegardes du profil (ou Sauvegarde > Restaurer des fichiers…) : « Restaurer des fichiers » montre une seule arborescence de tous les fichiers et dossiers jamais sauvegardés ; les fichiers qui ne sont plus à la source sont marqués { $deleted-icon } supprimé et peuvent encore être restaurés. Cochez des fichiers ou des dossiers dans l'arborescence ou dans les résultats de recherche (le champ de recherche trouve un fichier par son nom).
doc-restore-versions = Un fichier coché revient dans sa version la plus récente ; un dossier coché revient en entier, tel qu’il était dans la sauvegarde la plus récente qui le contient.
doc-restore-where = En haut du panneau de droite, choisissez où tout va : à côté de l'original avec la date dans le nom (rapport.docx revient sous le nom « rapport (2026-11-02 21.30).docx » et le fichier actuel n'est pas modifié), à l'emplacement d'origine ou dans un autre dossier ; les fichiers existants ne sont remplacés que si vous cochez « Remplacer les fichiers qui existent déjà ». Restaurer ramène ensuite tout en une seule opération. « Effacer la sélection », à côté, décoche tout.
doc-verify-title = Contrôle d'intégrité et test de restauration
doc-verify-body = La vérification complète relit chaque fichier stocké de chaque sauvegarde et le compare à son empreinte ; le test de restauration relit au hasard quelques fichiers de la sauvegarde la plus récente, comme le ferait une restauration, sans les écrire nulle part. Lancez-les depuis la section Vérifications des paramètres du profil (ou Sauvegarde > Vérifier les sauvegardes maintenant), ou faites-les exécuter tous les quelques jours après une sauvegarde planifiée. Si des problèmes sont détectés, faites une sauvegarde complète et vérifiez le disque de destination.
doc-activity-title = Onglet Activité
doc-activity-body = L'onglet Activité montre la progression de la sauvegarde, de la fusion, de la restauration ou de la vérification en cours, avec un bouton Annuler, ainsi que le journal de chaque opération passée, automatique ou lancée manuellement : ce qu'elle a fait, et quels fichiers ont été ignorés parce qu'un autre programme les avait ouverts. Au-dessus des étapes d'une opération, Ouvrir le journal complet affiche tout son journal sous forme de fichier texte dans votre éditeur : les étapes, puis chaque fichier traité avec l'heure, ce qui lui est arrivé (nouveau, modifié, enregistré sous forme de modifications, copié à nouveau, supprimé, déplacé, version retirée, restauré, conservé, ignoré, problème), sa taille et son chemin complet ; les fichiers inchangés d'une sauvegarde incrémentielle et les fichiers intacts d'une vérification complète sont seulement comptés. Copier le journal copie le même texte ; Journal du programme ouvre le journal technique d'Archeion, pour les problèmes plus profonds.
doc-window-title = Fermeture de la fenêtre
doc-window-body = Fermer la fenêtre n'arrête pas Archeion : il passe dans la zone de notification, à côté de l'horloge, et continue de travailler, donc les anciennes sauvegardes sont toujours fusionnées ; rouvrez la fenêtre depuis son icône. Archeion démarre aussi avec Windows, masqué à cet endroit. Fichier > Quitter, ou Quitter dans le menu de l'icône, le ferme vraiment. Ces deux comportements peuvent être désactivés dans Fichier > Préférences, où vous choisissez aussi la langue. Les sauvegardes planifiées s'exécutent dans tous les cas, même quand Archeion n'est pas lancé.
doc-cli-title = Ligne de commande
doc-cli-body = archeion.exe sans arguments ouvre la fenêtre. Les commandes ci-dessous exécutent une opération sans la fenêtre, comme le font les tâches planifiées. L'identifiant du profil est l'id du profil dans settings.toml.
doc-cli-run = Sauvegarde le profil (--kind complete pour une sauvegarde complète).
doc-cli-merge = Fusionne les anciennes sauvegardes selon les règles du profil.
doc-cli-verify = Vérifie les sauvegardes (--sample N : test de restauration sur N fichiers).
doc-cli-delete = Supprime une sauvegarde.
doc-cli-restore = Restaure les fichiers indiqués dans un fichier de demande écrit par la fenêtre.
doc-cli-watch = Signale les profils sans sauvegarde récente ; le Planificateur de tâches l'exécute chaque jour.
doc-cli-extract = Restaure à partir d'un dossier de sauvegardes seul, sans paramètres ni profil : par exemple avec l'Archeion portable sur un autre ordinateur. Les fichiers vont sous le dossier cible avec leur chemin complet.
doc-cli-extract-options = Options de extract : --archive <nom> choisit la sauvegarde (la plus récente par défaut) ; --path <fichier ou dossier> ne restaure que cela, indiqué par son chemin d'origine ; --overwrite remplace les fichiers existants. Pour des sauvegardes chiffrées, donnez le mot de passe ou la clé de récupération avec --stdin (première ligne de l'entrée standard), avec la variable ARCHEION_PASSWORD ou ARCHEION_RECOVERY_KEY, ou avec --password ou --recovery-key. La progression est aussi écrite dans archeion-extract.log dans le dossier cible. Dans l'invite de commandes, lancez-le avec start /wait archeion extract … pour que l'invite l'attende.
doc-cli-exit = Codes de sortie : 0 réussite, 1 échec, 2 destination introuvable, 3 occupé (une autre opération utilise la destination), 4 mot de passe manquant ou incorrect, 5 espace libre insuffisant.
doc-data-title = Emplacement des données et des journaux
doc-data-body = Les paramètres se trouvent dans le dossier %APPDATA%\Archeion\config (settings.toml), et le journal de chaque opération dans son sous-dossier logs. Une copie portable (avec un fichier portable.txt à côté d'archeion.exe) garde plutôt tout dans un dossier data à côté de l'exécutable. Aide > À propos d'Archeion ouvre ces dossiers.
doc-data-backups = Les sauvegardes elles-mêmes se trouvent uniquement sur la destination : un dossier par sauvegarde, nommé d'après sa date et son heure, qui contient les fichiers stockés et la liste de tous les fichiers à ce moment-là.

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
    [one] { $count } fichier trouvé
   *[other] { $count } fichiers trouvés
}
step-folders = { $count ->
    [one] { $count } dossier lu
   *[other] { $count } dossiers lus
}
step-new = { $count } nouveaux
step-changed = { $count } modifiés
step-unchanged = { $count } inchangés
step-copied-again = { $count } inchangés recopiés
step-moved = { $count ->
    [one] { $count } fichier déplacé
   *[other] { $count } fichiers déplacés
}
step-restored = { $count } restaurés
step-kept = { $count } déjà présents, conservés
step-checked = { $count } vérifiés
step-damaged = { $count ->
    [one] { $count } problème
   *[other] { $count } problèmes
}
step-skipped = { $count } ignorés
merge-group = { $count } sauvegardes, du { $first } au { $last }, en une seule
merge-group-of = Groupe { $index } sur { $of } : { $count } sauvegardes, du { $first } au { $last }, en une seule
recent-title = Derniers fichiers
recent-new = Nouveau
recent-changed = Modifié
recent-copied-again = Recopié
recent-moved = Déplacé
recent-restored = Restauré
recent-kept = Conservé
recent-checked = OK
recent-damaged = Problème
recent-skipped = Ignoré
recent-deleted = Supprimé
recent-dropped = Version retirée

# Profile files (File menu)
import-profile = Importer un profil…
export-profile = Exporter le profil…
import-profile-hint = Ajoute un profil depuis un fichier exporté par Archeion : ses règles, pas son mot de passe.
export-profile-hint = Enregistre toutes les règles de ce profil dans un fichier, pour le conserver ou l’emporter sur un autre ordinateur. Le mot de passe n’y est pas enregistré.
profile-file-type = Profil Archeion
import-failed-read = Impossible de lire le fichier : { $detail }
import-failed-format = { $file } n’est pas un profil Archeion.
import-failed-newer = { $file } a été créé par une version plus récente d’Archeion : mettez Archeion à jour pour l’importer.
export-failed = Impossible d’enregistrer le profil : { $detail }
doc-profiles-files = Fichier > Exporter le profil… enregistre toutes les règles du profil sélectionné (dossiers, exclusions, destination, format, planification, fusion, vérifications, notifications) dans un fichier .archeion-profile, pour le conserver ou l’emporter sur un autre ordinateur ; Fichier > Importer un profil… l’ajoute de nouveau. Le mot de passe n’est jamais dans le fichier : un profil chiffré le demande après l’importation. Sur un autre ordinateur, le profil importé retrouve ses sauvegardes à la destination ; importé deux fois sur le même, il devient une copie avec « (2) » dans son nom.
