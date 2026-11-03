# Menus
menu-file = Archivo
menu-backup = Copia de seguridad
menu-help = Ayuda
new-profile = Nuevo perfil
quit = Salir
restore = Restaurar archivos…
data-folder = Carpeta de datos

# Profile list and empty state
profiles = Perfiles
unnamed = (sin nombre)
new-profile-name = Nueva copia de seguridad
empty-title = Aún no hay copias de seguridad
empty-body = Crea un perfil: elige las carpetas que quieres guardar, dónde guardarlas y cuándo.
tab-settings = Configuración
tab-backups = Copias

# Profile form
profile-name = Nombre
folders = Carpetas que copiar
no-folders = Aún no hay carpetas: añade al menos una.
add-folder = Añadir carpeta…
remove-folder = Quitar esta carpeta de la copia
exclusions = Exclusiones
exclusions-hint = Un patrón se compara con la ruta completa, sin distinguir mayúsculas: * cualquier texto en un nombre, ** cualquier carpeta, ? un carácter. Por ejemplo **/node_modules, **/cache/**, **/~$*.
destination = Destino
dest-local = Carpeta o disco externo
dest-network = Carpeta de red
dest-hint = Usa una carpeta reservada a este perfil. Si el cifrado (más abajo) no está activado, cualquiera que pueda abrirla puede leer las copias.
dest-free = Espacio libre en el destino: { $free }
dest-foreign-profile = Esta carpeta contiene las copias del perfil "{ $name }": elige otra carpeta para este perfil.
dest-foreign-deleted = Esta carpeta contiene las copias de un perfil que ya no existe.
dest-adopt = Usar estas copias para este perfil
dest-adopt-hint = Las copias existentes pasan a ser de este perfil: la próxima copia las continúa y desde aquí se pueden restaurar y fusionar.
dest-adopt-failed = No se han podido asumir las copias: { $detail }
dest-foreign-hint = El destino contiene las copias de otro perfil: consulta la sección Destino.
run-foreign-destination = El destino contiene las copias de otro perfil. Abre la configuración del perfil: si ese perfil se eliminó, "Usar estas copias para este perfil" las asume.
browse = Examinar…
container = Formato
container-zip-per-file = Un zip por archivo
container-folder = Copias simples
container-zip-per-file-hint = Cada archivo se comprime por separado (informe.docx pasa a ser informe.docx.zip) en las mismas carpetas; se abre con el Explorador de archivos o con 7-Zip.
container-folder-hint = Los archivos se copian tal cual, con los mismos nombres, sin compresión.
large-files = Guardar los archivos grandes solo como cambios, desde
large-files-unit = MB
large-files-hint = Los archivos de al menos este tamaño se guardan por partes: tras un cambio, una copia guarda solo las partes que cambiaron. Las copias completas los conservan también enteros, así que se siguen abriendo con 7-Zip. Los archivos más pequeños se guardan como se indica arriba.
large-files-change-hint = Puedes cambiarlo cuando quieras: las copias siguientes siguen el nuevo límite y las ya hechas quedan como están.
large-files-encrypted = Cada archivo se guarda por partes, así que de un archivo modificado solo se guarda lo que cambió; las copias completas conservan también copias enteras.
schedule = Copia automática
schedule-manual = Solo cuando yo la inicie
schedule-daily = Cada día
schedule-weekly = Cada semana
at = a las
schedule-hint = Si el equipo está apagado a esa hora, se ejecuta en cuanto vuelva a encenderse.
merge = Fusionar copias antiguas
merge-hint = Las copias antiguas no se eliminan, sino que se fusionan con la siguiente que se conserva: cada fecha conservada sigue completa y se libera espacio. La primera copia completa nunca se fusiona automáticamente.
merge-automatic-hint = La fusión es automática: mientras Archeion está en ejecución, fusiona las copias antiguas según las reglas de abajo, nunca mientras una copia u otra operación las está usando.
retention-hint = Una copia se conserva si alguna regla la selecciona. Las copias de los días indicados en "Todas, durante días" nunca se fusionan.
keep-within = Todas, durante días
keep-last = Más recientes
keep-daily = Una al día, durante días
keep-weekly = Una a la semana, durante semanas
keep-monthly = Una al mes, durante meses
keep-yearly = Una al año, durante años
keep-everything = Todas las reglas a 0: se conservan todas las copias y no se fusiona nada.
back-up-now = Hacer copia ahora
merge-now = Fusionar ahora
merge-now-hint = Aplica ahora las reglas de fusión; la primera copia completa se queda como está.
busy-hint = Ya hay una copia o una fusión en curso.
fix-profile-hint = Completa primero el perfil.
delete-profile = Eliminar perfil
delete-profile-hint = Pide confirmación y luego quita el perfil y sus tareas programadas; las copias se quedan en el destino, salvo que elijas eliminarlas también.
delete-profile-title = ¿Eliminar el perfil "{ $name }"?
delete-profile-body = Se quitan el perfil y sus tareas programadas. Hasta que cierres el mensaje de la barra de estado, Deshacer los recupera.
delete-profile-password = La contraseña guardada se quita del Administrador de credenciales de Windows cuando ya no es posible deshacer: al cerrar ese mensaje, eliminar otro perfil o salir de Archeion.
delete-profile-backups = Las copias de { $path } no se eliminan: un perfil nuevo que use esa carpeta puede asumirlas con "Usar estas copias para este perfil".
delete-profile-with-backups = Mover también las copias de { $path } a la Papelera de reciclaje
delete-profile-backups-gone = Se pueden restaurar desde la Papelera de reciclaje hasta que la vacíes, pero Deshacer ya no podrá recuperar el perfil. Solo se va lo que escribió Archeion: los demás archivos de esa carpeta se quedan. Una carpeta de red o una unidad extraíble no tienen Papelera: ahí no se mueve nada.
delete-profile-and-backups = Eliminar perfil, copias a la Papelera
delete-profile-body-backups = Se eliminan el perfil y sus tareas programadas, y sus copias van a la Papelera de reciclaje.
delete-backups-running = Moviendo las copias de "{ $name }" a la Papelera de reciclaje…
delete-backups-done = Las copias de "{ $name }" están en la Papelera de reciclaje.
delete-backups-left = Las copias de "{ $name }" están en la Papelera de reciclaje; { $path } se queda, con { $count ->
    [one] 1 elemento
   *[other] { $count } elementos
} que Archeion no escribió.
delete-backups-failed = No todas las copias de "{ $name }" pudieron ir a la Papelera de reciclaje: { $detail }
delete-backups-no-bin = { $path } no tiene Papelera de reciclaje (carpeta de red o unidad extraíble): las copias de "{ $name }" no se tocaron. Elimínalas a mano si ya no las necesitas.

day-monday = lunes
day-tuesday = martes
day-wednesday = miércoles
day-thursday = jueves
day-friday = viernes
day-saturday = sábado
day-sunday = domingo

error-empty-name = Ponle un nombre al perfil.
error-no-sources = Añade al menos una carpeta que copiar.
error-no-destination = Elige dónde guardar las copias.
error-destination-in-source = El destino está dentro de una carpeta que se copia; elige otro.

# Backups tab
refresh = Actualizar
reading-backups = Leyendo las copias…
no-backups-yet = Aún no hay copias de este perfil.
kind-complete = Completa
kind-incremental = Incremental

# Status bar
ready = Listo
cancel = Cancelar
undo = Deshacer
phase-starting = Iniciando…
phase-opening = Abriendo el destino…
phase-scanning = Buscando cambios…
phase-copying = Copiando…
phase-merging = Fusionando copias antiguas…
phase-restoring = Restaurando…
phase-verifying = Comprobando…
progress-of = { $done } de { $total }
skipped-count = { $count ->
    [one] { $count } archivo omitido (en uso)
   *[other] { $count } archivos omitidos (en uso)
}
profile-removed = Perfil "{ $name }" eliminado.
run-finished = Copia de "{ $name }" hecha: { $new } nuevos, { $changed } modificados, { $deleted } eliminados, { $added } copiados.
merge-finished = "{ $name }" fusionado: de { $before } a { $after } copias, { $freed } liberados.
restore-finished = { $count } archivos restaurados.
verify-finished = { $count } archivos comprobados, { $problems } problemas.
run-destination-missing = No se encuentra el destino: { $location }. Conecta el disco y vuelve a intentarlo.
run-busy = Hay otra copia o fusión de este perfil en curso; vuelve a intentarlo cuando termine.
run-failed = La operación ha fallado: { $detail }
run-no-space = No hay espacio libre suficiente en el destino: se necesitan { $needed } y hay { $available } libres. Libera espacio, o fusiona o elimina copias antiguas, y vuelve a ejecutarla: los archivos ya copiados se conservan y no se vuelven a copiar.
restore-no-space = No hay espacio libre suficiente donde se restauran los archivos: se necesitan { $needed } y hay { $available } libres. No se ha escrito nada: libera espacio, o restaura menos archivos o en otro disco.
backup-resumed = { $count ->
    [one] { $count } archivo conservado de la copia interrumpida
   *[other] { $count } archivos conservados de la copia interrumpida
}
backup-as-changes = { $count ->
    [one] { $count } archivo guardado solo como cambios
   *[other] { $count } archivos guardados solo como cambios
}
run-cancelled = Cancelado. No se ha cambiado nada en las copias; una copia detenida continúa la próxima vez desde donde se quedó.

# Errors
settings-unreadable = No se puede leer el archivo de configuración, así que los cambios no se guardarán: { $detail }
save-failed = No se ha podido guardar la configuración: { $detail }
schedule-failed = No se ha podido programar la ejecución automática: { $detail }
start-failed = No se ha podido iniciar la operación: { $detail }
unexpected-stop = La operación se ha detenido de forma inesperada.

# Restore browser
dates = Fechas de las copias
archive-tooltip = { $files } archivos, { $stored } guardados en esta copia ({ $size })
search-hint = Busca un archivo o una carpeta por nombre en todas las copias…
no-results = Ningún archivo ni carpeta con ese nombre.
results = { $count ->
    [one] { $count } archivo encontrado
   *[other] { $count } archivos encontrados
}
close = Cerrar
restore-to = Restaurar en:
target-beside = Junto al original, con la fecha
target-beside-hint = informe.docx se restaura como "informe (2026-11-02 21.30).docx": el archivo actual no se toca.
target-original = Ubicación original
target-folder = Otra carpeta
replace-existing = Reemplazar los archivos que ya existen
replace-warning = Los archivos actuales se reemplazarán por la copia de seguridad.
tick-files-hint = Marca al menos un archivo o carpeta.
choose-folder-hint = Elige la carpeta donde restaurar.
restore-selected = Restaurar
clear-selection = Borrar selección
tick-folder = Restaurar esta carpeta y todo su contenido
tick-file = Restaurar este archivo
backups-restore-files = Restaurar archivos
backups-restore-files-hint = Todos los archivos guardados alguna vez: marca lo que quieres recuperar, elige las versiones o la fecha y adónde va.
backups-manage = Gestionar copias
backups-manage-hint = La lista de fechas de las copias: fusiona varias en una o elimina una.
restore-tree-count = { $files ->
        [one] 1 archivo
       *[other] { $files } archivos
    } en las copias · eliminados en el origen: { $deleted }
restore-tree-hint = Marca archivos o carpetas para restaurarlos; la búsqueda encuentra un archivo o una carpeta por nombre, con mayúsculas o minúsculas. Los archivos marcados con { $deleted-icon } eliminado ya no están en el origen y aún se pueden restaurar.
deleted-label = eliminado
tree-deleted-file = No está en la copia más reciente: eliminado en el origen. Márcalo para restaurar una de sus versiones.
tree-deleted-folder = No está en la copia más reciente: eliminada en el origen. Márcala para restaurarla tal como estaba en una fecha.
tick-in-folder = Se restaura con la carpeta marcada { $folder }. Desmárcalo para dejarlo fuera.
tick-not-in-folder-date = No está en la copia con la que vuelve la carpeta marcada { $folder }: no se restaura. Desmarca esa carpeta para elegirlo por separado.
tick-partial = Hay algo marcado dentro, o excluido de la carpeta marcada. Haz clic para marcar toda la carpeta.
tick-full = Todo lo que contiene está marcado uno por uno. Haz clic para desmarcarlo todo.
tick-left-out = Excluido de la carpeta marcada { $folder }: no se restaura. Márcalo para restaurarlo con la carpeta.
selection-empty = Todavía no hay nada que restaurar. Marca archivos o carpetas en el árbol.
restore-to-hint = Adónde va todo lo marcado.
restore-conflict = Dos elementos marcados reemplazarían el mismo archivo original: desmarca uno de ellos.
restore-conflict-hint = Dos elementos marcados reemplazarían el mismo archivo original: desmarca uno de ellos.
restore-wait-versions = Leyendo las versiones de los archivos marcados…
restore-summary = { $files ->
        [one] 1 archivo
       *[other] { $files } archivos
    }, { $folders ->
        [one] 1 carpeta
       *[other] { $folders } carpetas
    } — { $size }

# Manual merge
merge-choose = Elegir copias para fusionar…
merge-choose-hint = Marca la primera y la última copia: todo lo que hay entre ellas se convierte en una sola copia.
merge-tick = Marcar como extremo del intervalo que fusionar
merge-tick-hint = Marca la copia más antigua y la más reciente que fusionar; las intermedias se incluyen.
merge-summary = { $count } copias se convierten en una, con fecha { $date }.
merge-complete-warning = El intervalo incluye la primera copia completa: se sustituye por una nueva copia completa con la fecha de la más reciente elegida, y las fechas anteriores ya no se podrán restaurar.
merge-complete-ack = Lo entiendo
merge-two-hint = Marca al menos dos copias.
merge-ack-hint = Confirma que entiendes la advertencia.
merge-selected = Fusionar

# Activity tab
tab-activity = Actividad
show-details = Detalles
cancel-hint = Detener ahora: las copias quedan como estaban antes de esta ejecución. La próxima copia reutiliza los archivos ya copiados.
activity-backup-running = Copiando "{ $name }"
activity-merge-running = Fusionando las copias de "{ $name }"
activity-restore-running = Restaurando desde "{ $name }"
activity-preparing = Preparando…
activity-file = Archivo
activity-folder = Carpeta
activity-files = Archivos
activity-speed = Velocidad
activity-elapsed = Transcurrido
activity-remaining = Restante
count-of = { $done } de { $total }
files-found = { $count } encontrados hasta ahora
about = unos { $time }
activity-took = Duración: { $time }.
activity-backup-summary = Hecho: { $new } nuevos, { $changed } modificados, { $deleted } eliminados, { $unchanged } sin cambios; { $copied } copiados.
activity-merge-summary = Hecho: de { $before } a { $after } copias, { $freed } liberados.
activity-restore-summary = Hecho: { $restored } archivos restaurados, { $kept } ya presentes y conservados.
skipped-hint = No se han podido leer estos archivos, normalmente porque otro programa los tiene abiertos. Su versión anterior, si la hay, sigue en las copias.
skipped-more = …y { $count } más (consulta el registro).

# Exclusions and format lock
no-exclusions = Nada excluido: se copian todos los archivos de las carpetas.
remove-exclusion = Volver a copiar esto
exclusion-folder = Carpeta y todo su contenido
exclusion-file = Archivo
exclusion-type = Todos los archivos de este tipo
exclusion-pattern = Patrón
exclude-folder = Excluir carpeta…
exclude-file = Excluir archivo…
exclude-type-hint = .tmp
exclude-type = Excluir tipo
common-types = Habituales:
advanced = Avanzado
exclude-pattern = Excluir patrones
exclude-pattern-add = Añadir
container-locked = El destino ya contiene copias en este formato: no se puede cambiar. Para usar el otro, elige un destino vacío.
container-unreachable = No se puede acceder al destino, así que ahora no se puede cambiar el formato.
container-checking = Comprobando el destino…
activity-idle-title = Nada en curso
activity-idle-body = Las copias, fusiones y restauraciones muestran aquí su progreso.

# Backup kinds and deletion
menu-run-incremental = Copia incremental ahora
menu-run-complete = Copia completa ahora
run-now = Ejecutar ahora:
run-incremental = Incremental
run-incremental-hint = Copia solo los archivos nuevos y modificados; rápida, y la opción habitual.
run-complete = Completa
run-complete-hint = Copia todos los archivos e inicia una nueva cadena: las copias posteriores ya no dependen de las anteriores. Tarda más y ocupa más espacio.
run-merge = Fusionar
run-first = Hacer la primera copia
run-incremental-button = Hacer copia incremental
run-complete-button = Hacer copia completa
full-backup = Copia completa:
full-never = Solo la primera
full-every-backups = Cada
full-every-days = Cuando la última tenga más de
full-backups-unit = copias
full-days-unit = días
full-backup-hint = Se aplica a las copias automáticas; una copia completa inicia una nueva cadena, de modo que restaurar no depende de las copias más antiguas.
run-finished-complete = Copia completa de "{ $name }" hecha: { $new } nuevos, { $changed } modificados, { $deleted } eliminados, { $added } copiados.
activity-complete-summary = Copia completa hecha: { $new } nuevos, { $changed } modificados, { $deleted } eliminados, { $unchanged } sin cambios; { $copied } copiados.
activity-delete-running = Eliminando una copia de "{ $name }"
activity-delete-summary = Hecho: de { $before } a { $after } copias, { $freed } liberados.
delete-finished = Se ha eliminado una copia de "{ $name }".
delete-backup = Eliminar esta copia…
delete-backup-hint = Quita la fecha seleccionada; las copias posteriores siguen completas.
delete-backup-confirm = Eliminar copia
delete-initial-hint = La primera copia completa solo se puede eliminar cuando es la única: las demás dependen de ella.
delete-complete-hint = Una copia completa inicia una cadena de la que dependen las copias posteriores: no se puede eliminar. Fusiónala con las siguientes.
delete-only-warning = La copia del { $date } es la única: el destino quedará vacío y la próxima copia será completa. Esto no se puede deshacer.
delete-newest-warning = Se eliminará la copia del { $date }, junto con las versiones de archivos guardadas solo en ella. Esto no se puede deshacer.
delete-middle-warning = Se eliminará la copia del { $date }. Los archivos que necesitan las copias posteriores pasan a la siguiente; las versiones guardadas solo en esta se pierden. Esto no se puede deshacer.

# Encryption
encrypt = Cifrar las copias (AES-256, incluidos los nombres de archivo)
encrypt-hint = Cada archivo es un zip AES-256 con el nombre oculto; sin la contraseña nadie puede leer el contenido ni ver qué archivos se guardan. La contraseña se guarda en el Administrador de credenciales de Windows.
encrypt-needs-zip = El cifrado necesita el formato "Un zip por archivo".
password = Contraseña
password-again = Repite la contraseña
password-set = Elige la contraseña
password-reenter = Escribe la contraseña de estas copias
password-saved = Contraseña guardada en el Administrador de credenciales de Windows.
password-missing = No hay ninguna contraseña guardada en este equipo: escríbela para hacer copias y explorarlas.
password-loss-warning = Sin esta contraseña, o la clave de recuperación que se muestra tras la primera copia, las copias no se pueden abrir y nadie puede recuperarlas. Guarda la contraseña en un lugar seguro.
password-save = Guardar contraseña
password-checking = Comprobando la contraseña…
password-empty-hint = Escribe la contraseña.
password-short-hint = Usa al menos { $count } caracteres.
password-mismatch-hint = Las dos contraseñas no coinciden.
password-rule-length = Al menos { $count } caracteres (ahora { $typed })
password-rule-match = Las dos contraseñas coinciden
password-rule-variety = Al menos { $kinds } tipos de caracteres entre minúsculas, mayúsculas, dígitos y símbolos (no hace falta a partir de { $passphrase } caracteres)
password-rule-common = No es una contraseña habitual, como "Password2024" o "qwerty"
password-rule-runs = Sin { $count } caracteres repetidos o consecutivos, como "aaaa", "1234" o "abcd"
password-rule-name = No contiene el nombre del perfil
password-rule-strength = Lo bastante difícil de adivinar: seguridad mínima { $level }
password-rule-tip = Sirve cualquier carácter, espacios incluidos: una frase de cuatro o cinco palabras es fácil de recordar y difícil de adivinar, y a partir de { $passphrase } caracteres no necesita dígitos, mayúsculas ni símbolos.
password-strength = Seguridad: { $level }
password-strength-weak = Débil
password-strength-fair = Aceptable
password-strength-strong = Fuerte
password-strength-very-strong = Muy fuerte
password-variety-hint = Combina minúsculas, mayúsculas, dígitos y símbolos, o escribe una frase de al menos { $passphrase } caracteres.
password-common-hint = Es una de las contraseñas más usadas: elige otra.
password-runs-hint = Evita secuencias como "aaaa" o "1234".
password-name-hint = No pongas el nombre del perfil en la contraseña.
password-weak-hint = Demasiado fácil de adivinar: hazla más larga, por ejemplo con unas cuantas palabras más.
password-first-hint = Escribe arriba la contraseña de las copias cifradas (dos veces para copias nuevas, y marca "Lo entiendo"): el botón la guarda e inicia la copia.
password-wrong = Esta contraseña no abre las copias del destino.
password-unreachable = No se ha podido comprobar la contraseña
run-password-needed = Las copias están cifradas y no hay ninguna contraseña guardada en este equipo: escríbela en la configuración del perfil.
run-wrong-password = La contraseña guardada no abre las copias: vuelve a escribirla en la configuración del perfil.

# Operations log
log-runs = Operaciones
log-only-profile = Solo este perfil
log-running = En curso
log-empty = Aún no hay operaciones.
log-pick = Elige una operación a la izquierda.
log-title = Registro
log-no-lines = Aún no hay nada registrado.
log-scheduled = automática
log-manual = iniciada a mano
log-started = Iniciada { $time }
log-backup = copia { $name }
log-unfinished = No terminó: se interrumpió o sigue en curso.
log-skipped = Omitido { $path }: { $reason }
log-op-backup = Copia de seguridad
log-op-incremental = Copia incremental
log-op-complete = Copia completa
log-op-merge = Fusión
log-op-delete = Eliminación de copia
log-op-restore = Restauración
log-open-full = Abrir registro completo
log-open-full-hint = Escribe el registro completo de esta operación en un archivo de texto y lo abre en tu editor de texto: cada paso y después cada archivo con el que trabajó y lo que hizo con él, incluidos los omitidos.
log-copy = Copiar registro
log-copy-hint = Copia el registro completo de esta operación como texto, para pegarlo en un correo o en un informe de problemas.
log-copied = Registro copiado.
log-open-program = Registro del programa
log-open-program-hint = Abre archeion.log, el registro técnico del programa: útil para investigar un problema o para informar de él.
log-open-folder = Carpeta de registros
log-open-folder-hint = Abre la carpeta de los registros de operaciones en el Explorador de archivos.
log-program-missing = El registro del programa aún no existe.
log-unreadable = No se puede leer el archivo de registro.
log-report-failed = No se puede mostrar el registro completo: { $detail }
log-level-info = Info
log-level-done = OK
log-level-warning = Advertencia
log-level-error = Error
log-preparing = Preparando el registro completo…
log-files-title = Archivos
log-files-missing = La lista de archivos no se registró para esta operación: la hizo una versión anterior de Archeion.
log-files-none = No se procesó ningún archivo.
log-file-new-chunks = guardado como cambios: { $size } de bloques nuevos escritos
log-file-resumed = ya copiado por la copia interrumpida, conservado
log-count-new = Nuevos: { $count }
log-count-changed = Modificados: { $count }
log-count-as-changes = Guardados como cambios: { $count }
log-count-copied-again = Copiados de nuevo: { $count }
log-count-unchanged = Sin cambios, no listados: { $count }
log-count-deleted = Eliminados: { $count }
log-count-moved = Movidos: { $count }
log-count-dropped = Versiones eliminadas: { $count }
log-count-restored = Restaurados: { $count }
log-count-kept = Conservados: { $count }
log-count-checked = Releídos intactos: { $count }
log-count-checked-unlisted = Releídos intactos, no listados: { $count }
log-count-damaged = Problemas: { $count }
log-count-skipped = Omitidos: { $count }

# Password change and recovery key
password-change = Cambiar la contraseña
password-new = Nueva contraseña
password-change-button = Cambiar contraseña
recovery-later = La clave de recuperación estará disponible después de la primera copia.
recovery-show = Mostrar la clave de recuperación…
recovery-title = Clave de recuperación
recovery-hint = Con esta clave, 7-Zip abre todos los archivos y manifiestos cifrados, incluso sin Archeion y sin la contraseña. El manifiesto de cada copia indica qué archivo está guardado dónde.
recovery-warning = Quien tenga esta clave puede leer todas las copias. Imprímela o anótala y guárdala sin conexión; no cambia cuando cambias la contraseña.
recovery-copy = Copiar
recovery-copy-hint = Copia la clave sin espacios, como la quiere 7-Zip.
recovery-hide = Ocultar
password-current = Contraseña actual
password-current-hint = Escribe la contraseña actual.
recovery-ask = Para mostrar la clave de recuperación, escribe la contraseña actual de estas copias.
recovery-show-button = Mostrar

# Integrity check
verify-section = Comprobaciones
verify-hint = La comprobación completa vuelve a leer cada archivo guardado de cada copia y lo compara con su huella; la prueba de restauración vuelve a leer al azar algunos archivos de la copia más reciente. Las comprobaciones pendientes se ejecutan después de una copia programada.
verify-full-every = Comprobación completa cada
verify-sample-every = Prueba de restauración cada
verify-sample-files = en archivos:
verify-zero-hint = 0 días = nunca automáticamente.
verify-full-now = Comprobar copias ahora
verify-full-hint = Vuelve a leer cada archivo guardado y lo comprueba; en un destino grande tarda lo mismo que leer todas las copias.
verify-sample-now = Prueba de restauración ahora
verify-sample-hint = Vuelve a leer al azar algunos archivos de la copia más reciente, como haría una restauración, sin escribirlos en ningún sitio.
verify-no-backups = Aún no hay copias que comprobar.
verify-problems = Problemas encontrados
verify-problems-hint = Puede que las copias indicadas no se restauren correctamente. Haz ahora una copia completa y revisa el disco de destino.
activity-verify-running = Comprobando las copias de "{ $name }"
log-op-verify = Comprobación de integridad
log-op-restore-test = Prueba de restauración

# Automatic backup: schedules, events, notifications
schedule-days = Los días elegidos
schedule-hours = Cada pocas horas
schedule-monthly = Una vez al mes
schedule-all-days = Todos los días
schedule-add-time = + Añadir una hora
schedule-remove-time = Quitar esta hora
schedule-every = Cada
hours-unit = horas
schedule-from = desde
schedule-until = hasta
schedule-hours-hint = La última ejecución empieza antes de la hora de fin; la misma hora de inicio y fin significa todo el día.
schedule-month-day = El día
schedule-month-hint = Solo los días del 1 al 28, para que todos los meses tengan ese día.
events-title = Hacer copia también:
event-logon = Unos minutos después de iniciar sesión en Windows
event-idle = Cuando el equipo lleve 10 minutos inactivo
event-drive = Cuando se conecte una unidad
event-drive-hint = Útil con un disco externo como destino: la copia empieza al conectarlo, y no pasa nada cuando se conecta otra unidad.
event-min-hours = No más a menudo que cada
event-hint = Una copia iniciada por uno de estos eventos se omite si la última copia es más reciente o si el destino no está conectado.
alerts = Notificaciones
alert-on-failure = Avisarme cuando falle una copia automática
alert-stale = Avisarme cuando no haya copias desde hace
alert-hint = Notificaciones de Windows; 0 días = sin aviso. La comprobación se hace cada día y después de iniciar sesión.
alert-manual-hint = Solo para copias automáticas: elige una programación en "Copia automática".
alert-failed-title = Archeion: ha fallado la copia de "{ $name }"
alert-stale-title = Archeion: no hay copia de "{ $name }"
alert-stale-body = Sin copias desde hace { $days } días (la última, el { $when }). Comprueba que el destino esté conectado o inicia una copia desde Archeion.
closing-after-run = Hay una operación en curso: Archeion se cerrará solo cuando termine.
closing-cancel = No cerrar

# Notification area and Preferences window
menu-preferences = Preferencias…
preferences-title = Preferencias
preferences-window = Ventana
language = Idioma
language-system = Idioma del sistema
language-tray-restart = El menú del icono del área de notificación cambiará de idioma la próxima vez que se inicie Archeion.
quit-hint = Cierra Archeion; las copias programadas se siguen ejecutando desde el Programador de tareas de Windows.
tray-open = Abrir Archeion
tray-quit = Salir
tray-running = operación en curso
option-tray = Seguir en el área de notificación al cerrar la ventana
option-tray-hint = Al cerrar la ventana, se oculta junto al reloj: Archeion sigue fusionando las copias antiguas y terminando la operación en curso. Haz clic en su icono para volver a abrirla o elige Salir.
option-tray-unavailable = Windows no ha proporcionado un icono en el área de notificación.
option-autostart = Iniciar Archeion con Windows
option-autostart-hint = Al iniciar sesión, Archeion se inicia oculto en el área de notificación. También puedes desactivarlo en el Administrador de tareas, en Aplicaciones de arranque.
autostart-failed = No se ha podido cambiar el inicio con Windows: { $detail }

# Help menu: user guide and About
menu-documentation = Documentación
menu-about = Acerca de Archeion
about-version = Versión { $version }
about-name = El nombre viene del griego antiguo ἀρχεῖον, "archivo".
about-author = Autor: MR
about-license = Licencia: GPL-3.0
about-repository = Código fuente en GitHub
about-logs-folder = Carpeta de registros
about-open-folder = Abrir carpeta
about-folder-missing = La carpeta aún no existe.
doc-title = Documentación de Archeion
doc-intro = Archeion guarda copias de tus carpetas en otro disco, en una carpeta de red o en cualquier otra carpeta, y te devuelve cualquier archivo tal como estaba en una fecha pasada. Abre una sección para leerla.
doc-what-title = Qué hace Archeion
doc-what-backups = La primera copia de un perfil es completa: copia todos los archivos. Las siguientes son incrementales: copian solo los archivos nuevos y modificados, así que son rápidas y pequeñas, y aun así cada fecha se puede restaurar entera. En "Copia completa", en la configuración del perfil, también puedes pedir una nueva copia completa cada cierto número de copias, o cuando la última tenga más de unos días; una copia completa inicia una nueva cadena que ya no depende de las copias más antiguas.
doc-what-storage = Cada archivo se guarda por separado, en las mismas carpetas que el original: como un zip de ese archivo ("Un zip por archivo": informe.docx pasa a ser informe.docx.zip), como una copia simple o como un zip cifrado con AES-256. Las copias se pueden leer sin Archeion: el Explorador de archivos y 7-Zip abren los zip, y 7-Zip abre los cifrados con la clave de recuperación.
doc-what-safe = Una copia interrumpida (equipo apagado, disco desconectado, Cancelar) deja las copias existentes como estaban; la siguiente copia continúa desde donde se quedó y no vuelve a copiar los archivos ya copiados.
doc-profiles-title = Perfiles
doc-profiles-body = Un perfil indica qué copiar, dónde y cuándo. Crea uno con Archivo > Nuevo perfil (Ctrl+N) y rellena su pestaña Configuración: un nombre, las carpetas que copiar y el destino, que puede ser una carpeta, un disco externo o una carpeta de red. Usa cada carpeta de destino para un solo perfil.
doc-profiles-exclusions = Las exclusiones dejan fuera lo que no necesitas: una carpeta y todo su contenido, un archivo o todos los archivos de un tipo, como .tmp. Con "Excluir patrones" puedes añadir patrones, que se comparan con la ruta completa sin distinguir mayúsculas: * es cualquier texto en un nombre, ** cualquier carpeta, ? un carácter. Por ejemplo **/node_modules o **/cache/**.
doc-profiles-format = El formato de almacenamiento (un zip por archivo o copias simples) y el cifrado deben elegirse antes de la primera copia: cuando el destino ya contiene copias, no se pueden cambiar. Para cambiarlos, elige un destino vacío.
doc-profiles-delete = Eliminar perfil pide confirmación y luego quita el perfil y sus tareas programadas; las copias se quedan en el destino. Justo después, Deshacer en la barra de estado recupera el perfil. Un perfil que use esa carpeta más adelante puede asumirlas con "Usar estas copias para este perfil". En la confirmación, "Mover también las copias a la Papelera de reciclaje" las mueve allí también, sin Deshacer para el perfil: solo se va lo que Archeion escribió en esa carpeta, y la carpeta entera cuando no contiene nada más. Una carpeta de red o una unidad extraíble no tienen Papelera: ahí no se mueve nada.
doc-auto-title = Copias automáticas
doc-auto-schedule = En "Copia automática" elige cuándo se hacen las copias solas: los días elegidos a una o varias horas, cada pocas horas entre dos horas, o una vez al mes un día del 1 al 28. Archeion registra la programación en el Programador de tareas de Windows, así que las copias se hacen aunque Archeion no esté abierto.
doc-auto-missed = Si el equipo está apagado a la hora programada, la copia se ejecuta en cuanto vuelva a encenderse.
doc-auto-alerts = En "Notificaciones", Archeion puede mostrar una notificación de Windows cuando falla una copia automática, y un aviso cuando un perfil lleva cierto número de días sin copia (se comprueba cada día y después de iniciar sesión).
doc-auto-now = Siempre puedes iniciar una copia tú mismo: Hacer copia ahora en la configuración del perfil, o Copia de seguridad > Copia incremental ahora (Ctrl+B) y Copia completa ahora.
doc-merge-title = Fusión de copias antiguas
doc-merge-body = Las copias antiguas no se eliminan, sino que se fusionan con la siguiente copia que se conserva: cada fecha conservada sigue completa y se libera el espacio de las versiones que ya no hacen falta. La fusión es automática: se hace mientras Archeion está en ejecución, nunca mientras una copia u otra operación usa el destino.
doc-merge-rules = Las reglas de "Fusionar copias antiguas" indican qué copias conservar, y una copia se conserva si alguna regla la selecciona: todas las copias de los últimos días, las más recientes, una al día, una a la semana, una al mes y una al año, cada una durante la cantidad que indiques. Con todas las reglas a 0, se conservan todas las copias y no se fusiona nada.
doc-merge-first = La primera copia completa nunca se fusiona automáticamente.
doc-merge-manual = En Copias > Gestionar copias, "Elegir copias para fusionar…" fusiona a mano: marca la copia más antigua y la más reciente, y todo lo que hay entre ellas se convierte en una sola copia. Si el intervalo incluye la primera copia completa, Archeion te avisa antes, porque las fechas anteriores ya no se podrán restaurar.
doc-merge-delete = "Eliminar esta copia…" en Copias > Gestionar copias quita una fecha; las copias posteriores siguen completas. Una copia completa de la que dependen copias posteriores no se puede eliminar (fusiónala en su lugar). Una eliminación no se puede deshacer.
doc-encrypt-title = Cifrado
doc-encrypt-body = Con "Cifrar las copias", cada archivo se convierte en un zip AES-256 con el nombre oculto: sin la contraseña nadie puede leer el contenido ni ver qué archivos se guardan. El cifrado necesita el formato "Un zip por archivo" y debe activarse antes de la primera copia; después no se puede cambiar.
doc-encrypt-password = La contraseña se guarda en el Administrador de credenciales de Windows, así que las copias automáticas no necesitan que la escribas. Puedes cambiarla más adelante en la configuración del perfil, escribiendo la actual. En otro equipo, escríbela una vez en la configuración del perfil.
doc-encrypt-recovery = Después de la primera copia, "Mostrar la clave de recuperación…" muestra una clave con la que 7-Zip abre todos los archivos cifrados, incluso sin Archeion y sin la contraseña. Imprímela o anótala y guárdala sin conexión: sin la contraseña y sin la clave nadie puede abrir las copias. La clave no cambia cuando cambias la contraseña. Archeion pide la contraseña actual antes de mostrarla.
doc-large-title = Archivos grandes
doc-large-body = Un archivo grande que cambia poco (un buzón de correo, el disco de una máquina virtual, una base de datos) no se vuelve a copiar entero: se guarda en partes de alrededor de 1 MB y cada copia guarda solo las partes nuevas. Las partes idénticas se guardan una sola vez. Se aplica a los archivos de al menos el tamaño elegido en la sección Formato del perfil (500 MB si no se cambia; se puede desactivar) y a todos los archivos de un destino cifrado.
doc-large-complete = Las copias completas siguen conservando cada archivo entero, como zip, copia simple o zip cifrado, así que se pueden restaurar solo con 7-Zip; solo las copias incrementales posteriores contienen partes. Al restaurar se vuelven a unir las partes y se comprueba cada una; la comprobación de integridad lee todas las partes.
doc-large-upgrade = Las copias hechas por versiones anteriores se quedan como están y se siguen pudiendo leer. Las partes empiezan con la siguiente copia completa: en un destino así, la próxima copia automática es completa.
doc-restore-title = Restaurar archivos
doc-restore-body = Abre la pestaña Copias del perfil (o Copia de seguridad > Restaurar archivos…): "Restaurar archivos" muestra un único árbol con todos los archivos y carpetas que las copias han guardado alguna vez; los archivos que ya no están en el origen aparecen marcados con { $deleted-icon } eliminado y aún se pueden restaurar. Marca archivos o carpetas en el árbol o en los resultados de búsqueda (el cuadro de búsqueda encuentra un archivo o una carpeta por nombre, con mayúsculas o minúsculas).
doc-restore-versions = Un archivo marcado vuelve en su versión más reciente; una carpeta marcada vuelve entera, tal como estaba en la copia más reciente que la contiene.
doc-restore-where = Arriba del panel derecho elige adónde va todo: junto al original con la fecha en el nombre (informe.docx vuelve como "informe (2026-11-02 21.30).docx" y el archivo actual no se toca), la ubicación original u otra carpeta; los archivos existentes solo se reemplazan si marcas "Reemplazar los archivos que ya existen". Restaurar lo recupera todo en una sola operación. "Borrar selección", a su lado, lo desmarca todo.
doc-verify-title = Comprobación de integridad y prueba de restauración
doc-verify-body = La comprobación completa vuelve a leer cada archivo guardado de cada copia y lo compara con su huella; la prueba de restauración vuelve a leer al azar algunos archivos de la copia más reciente, como haría una restauración, sin escribirlos en ningún sitio. Inícialas desde la sección Comprobaciones de la configuración del perfil (o Copia de seguridad > Comprobar copias ahora), o haz que se ejecuten cada pocos días después de una copia programada. Si se encuentran problemas, haz una copia completa y revisa el disco de destino.
doc-activity-title = Pestaña Actividad
doc-activity-body = La pestaña Actividad muestra el progreso de la copia, fusión, restauración o comprobación en curso, con un botón Cancelar, y el registro de cada operación anterior, automática o iniciada a mano: lo que hizo y qué archivos se omitieron porque otro programa los tenía abiertos. Encima de los pasos de una operación, Abrir registro completo muestra todo su registro como archivo de texto en tu editor: los pasos y después cada archivo con el que trabajó, con la hora, lo que le pasó (nuevo, modificado, guardado como cambios, copiado de nuevo, eliminado, movido, versión eliminada, restaurado, conservado, omitido, problema), su tamaño y su ruta completa; los archivos sin cambios de una copia incremental y los intactos de una comprobación completa solo se cuentan. Copiar registro copia el mismo texto; Registro del programa abre el registro técnico de Archeion, para problemas más complejos.
doc-window-title = Cerrar la ventana
doc-window-body = Cerrar la ventana no detiene Archeion: pasa al área de notificación, junto al reloj, y sigue trabajando, así que las copias antiguas se siguen fusionando; vuelve a abrir la ventana desde su icono. Archeion también se inicia con Windows, oculto allí. Archivo > Salir, o Salir en el menú del icono, lo cierra de verdad. Ambas cosas se pueden desactivar en Archivo > Preferencias, donde también eliges el idioma. Las copias programadas se ejecutan en cualquier caso, aunque Archeion no esté en ejecución.
doc-cli-title = Línea de comandos
doc-cli-body = archeion.exe sin argumentos abre la ventana. Los comandos siguientes ejecutan una operación sin la ventana, como hacen las tareas programadas. El id de perfil es el id del perfil en settings.toml.
doc-cli-run = Hace la copia del perfil (--kind complete para una copia completa).
doc-cli-merge = Fusiona las copias antiguas según las reglas del perfil.
doc-cli-verify = Comprueba las copias (--sample N: prueba de restauración de N archivos).
doc-cli-delete = Elimina una copia.
doc-cli-restore = Restaura los archivos indicados en un archivo de solicitud escrito por la ventana.
doc-cli-watch = Avisa de los perfiles sin una copia reciente; el Programador de tareas lo ejecuta cada día.
doc-cli-extract = Restaura desde una carpeta de copias por sí sola, sin configuración ni perfil: por ejemplo con el Archeion portátil en otro equipo. Los archivos van bajo la carpeta de destino con su ruta completa.
doc-cli-extract-options = Opciones de extract: --archive <nombre> elige la copia (la más reciente si se omite); --path <archivo o carpeta> restaura solo eso, indicado con su ruta original; --overwrite reemplaza los archivos existentes. Para copias cifradas, indica la contraseña o la clave de recuperación con --stdin (primera línea de la entrada estándar), con la variable ARCHEION_PASSWORD o ARCHEION_RECOVERY_KEY, o con --password o --recovery-key. El progreso también se escribe en archeion-extract.log en la carpeta de destino. En el Símbolo del sistema, ejecútalo como start /wait archeion extract … para que el símbolo del sistema lo espere.
doc-cli-exit = Códigos de salida: 0 correcto, 1 error, 2 destino no encontrado, 3 ocupado (otra operación está usando el destino), 4 contraseña ausente o incorrecta, 5 espacio libre insuficiente.
doc-data-title = Dónde están los datos y los registros
doc-data-body = La configuración está en la carpeta %APPDATA%\Archeion\config (settings.toml), y el registro de cada operación está en su subcarpeta logs. Una copia portable (con un archivo portable.txt junto a archeion.exe) guarda en cambio todo en una carpeta data junto al ejecutable. Ayuda > Acerca de Archeion abre estas carpetas.
doc-data-backups = Las copias en sí están solo en el destino: una carpeta por copia, con el nombre de su fecha y hora, que contiene los archivos guardados y una lista de todos los archivos en ese momento.

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
    [one] { $count } archivo encontrado
   *[other] { $count } archivos encontrados
}
step-folders = { $count ->
    [one] { $count } carpeta leída
   *[other] { $count } carpetas leídas
}
step-new = { $count } nuevos
step-changed = { $count } modificados
step-unchanged = { $count } sin cambios
step-copied-again = { $count } sin cambios copiados de nuevo
step-moved = { $count ->
    [one] { $count } archivo movido
   *[other] { $count } archivos movidos
}
step-restored = { $count } restaurados
step-kept = { $count } ya presentes, conservados
step-checked = { $count } comprobados
step-damaged = { $count ->
    [one] { $count } problema
   *[other] { $count } problemas
}
step-skipped = { $count } omitidos
merge-group = { $count } copias, del { $first } al { $last }, en una
merge-group-of = Grupo { $index } de { $of }: { $count } copias, del { $first } al { $last }, en una
recent-title = Últimos archivos
recent-new = Nuevo
recent-changed = Modificado
recent-copied-again = Copiado de nuevo
recent-moved = Movido
recent-restored = Restaurado
recent-kept = Conservado
recent-checked = OK
recent-damaged = Problema
recent-skipped = Omitido
recent-deleted = Eliminado
recent-dropped = Versión eliminada

# Profile files (File menu)
import-profile = Importar perfil…
export-profile = Exportar perfil…
import-profile-hint = Añade un perfil desde un archivo exportado por Archeion: sus reglas, no su contraseña.
export-profile-hint = Guarda todas las reglas de este perfil en un archivo, para conservarlo o llevarlo a otro equipo. La contraseña no se guarda en él.
profile-file-type = Perfil de Archeion
import-failed-read = No se pudo leer el archivo: { $detail }
import-failed-format = { $file } no es un perfil de Archeion.
import-failed-newer = { $file } lo creó una versión más reciente de Archeion: actualiza Archeion para importarlo.
export-failed = No se pudo guardar el perfil: { $detail }
doc-profiles-files = Archivo > Exportar perfil… guarda todas las reglas del perfil seleccionado (carpetas, exclusiones, destino, formato, programación, fusión, comprobaciones, notificaciones) en un archivo .archeion-profile, para conservarlo o llevarlo a otro equipo; Archivo > Importar perfil… lo vuelve a añadir. La contraseña nunca está en el archivo: un perfil cifrado la pide tras la importación. En otro equipo el perfil importado encuentra sus copias en el destino; importado dos veces en el mismo, se convierte en una copia con "(2)" en el nombre.
