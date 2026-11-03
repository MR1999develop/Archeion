# Menus
menu-file = Arquivo
menu-backup = Backup
menu-help = Ajuda
new-profile = Novo perfil
quit = Sair
restore = Restaurar arquivos…
data-folder = Pasta de dados

# Profile list and empty state
profiles = Perfis
unnamed = (sem nome)
new-profile-name = Novo backup
empty-title = Nenhum backup ainda
empty-body = Crie um perfil: escolha as pastas que serão salvas, onde salvá-las e quando.
tab-settings = Configurações
tab-backups = Backups

# Profile form
profile-name = Nome
folders = Pastas para backup
no-folders = Nenhuma pasta ainda: adicione pelo menos uma.
add-folder = Adicionar pasta…
remove-folder = Remover esta pasta do backup
exclusions = Exclusões
exclusions-hint = Um padrão é comparado com o caminho completo, sem diferenciar maiúsculas: * qualquer texto em um nome, ** quaisquer pastas, ? um caractere. Por exemplo **/node_modules, **/cache/**, **/~$*.
destination = Destino
dest-local = Pasta ou disco externo
dest-network = Compartilhamento de rede
dest-hint = Use uma pasta exclusiva para este perfil. Se a criptografia (abaixo) não estiver ativada, qualquer pessoa que consiga abri-la poderá ler os backups.
dest-free = Espaço livre no destino: { $free }
dest-foreign-profile = Esta pasta contém os backups do perfil "{ $name }": escolha outra pasta para este perfil.
dest-foreign-deleted = Esta pasta contém os backups de um perfil que não existe mais.
dest-adopt = Usar estes backups neste perfil
dest-adopt-hint = Os backups existentes passam a ser deste perfil: o próximo backup dá continuidade a eles, e eles podem ser restaurados e mesclados a partir daqui.
dest-adopt-failed = Não foi possível assumir os backups: { $detail }
dest-foreign-hint = O destino contém os backups de outro perfil: veja a seção Destino.
run-foreign-destination = O destino contém os backups de outro perfil. Abra as configurações do perfil: se esse perfil foi excluído, "Usar estes backups neste perfil" os assume.
browse = Procurar…
container = Formato
container-zip-per-file = Um zip por arquivo
container-folder = Cópias simples
container-zip-per-file-hint = Cada arquivo é compactado separadamente (relatorio.docx vira relatorio.docx.zip) nas mesmas pastas; ele abre no Explorador de Arquivos ou no 7-Zip.
container-folder-hint = Os arquivos são copiados como estão, com os mesmos nomes, sem compactação.
large-files = Salvar arquivos grandes só como alterações, a partir de
large-files-unit = MB
large-files-hint = Arquivos de pelo menos este tamanho são salvos em partes: depois de uma alteração, um backup guarda só as partes que mudaram. Os backups completos também os guardam inteiros, então continuam abrindo com o 7-Zip. Arquivos menores são salvos como acima.
large-files-change-hint = Você pode alterá-lo quando quiser: os próximos backups seguem o novo limite e os já feitos continuam como estão.
large-files-encrypted = Cada arquivo é salvo em partes, então de um arquivo alterado só se guarda o que mudou; os backups completos também guardam cópias inteiras.
schedule = Backup automático
schedule-manual = Só quando eu iniciar
schedule-daily = Todos os dias
schedule-weekly = Toda semana
at = às
schedule-hint = Se o computador estiver desligado nesse horário, o backup é feito assim que ele for ligado de novo.
merge = Mesclar backups antigos
merge-hint = Os backups antigos não são excluídos, mas mesclados ao próximo que é mantido; assim, cada data mantida continua completa e o espaço é liberado. O primeiro backup completo nunca é mesclado automaticamente.
merge-automatic-hint = A mesclagem é automática: enquanto o Archeion está em execução, ele mescla os backups antigos segundo as regras abaixo, nunca enquanto um backup ou outra operação os está usando.
retention-hint = Um backup é mantido se alguma regra o selecionar. Os backups dos dias definidos em "Todos, por dias" nunca são mesclados.
keep-within = Todos, por dias
keep-last = Mais recentes
keep-daily = Um por dia, por dias
keep-weekly = Um por semana, por semanas
keep-monthly = Um por mês, por meses
keep-yearly = Um por ano, por anos
keep-everything = Todas as regras em 0: todos os backups são mantidos e nada é mesclado.
back-up-now = Fazer backup agora
merge-now = Mesclar agora
merge-now-hint = Aplica as regras de mesclagem agora; o primeiro backup completo fica como está.
busy-hint = Já há um backup ou uma mesclagem em andamento.
fix-profile-hint = Primeiro complete o perfil.
delete-profile = Excluir perfil
delete-profile-hint = Pede confirmação e depois remove o perfil e suas tarefas agendadas; os backups permanecem no destino.
delete-profile-title = Excluir o perfil "{ $name }"?
delete-profile-body = O perfil e suas tarefas agendadas são removidos. Até você fechar a mensagem na barra de status, Desfazer os traz de volta.
delete-profile-password = A senha salva é removida do Gerenciador de Credenciais do Windows quando não for mais possível desfazer: ao fechar essa mensagem, excluir outro perfil ou sair do Archeion.
delete-profile-backups = Os backups em { $path } não são excluídos: um novo perfil que use essa pasta pode assumi-los com "Usar estes backups neste perfil".

day-monday = segunda-feira
day-tuesday = terça-feira
day-wednesday = quarta-feira
day-thursday = quinta-feira
day-friday = sexta-feira
day-saturday = sábado
day-sunday = domingo

error-empty-name = Dê um nome ao perfil.
error-no-sources = Adicione pelo menos uma pasta para backup.
error-no-destination = Escolha onde salvar os backups.
error-destination-in-source = O destino está dentro de uma pasta incluída no backup; escolha outro.

# Backups tab
refresh = Atualizar
reading-backups = Lendo os backups…
no-backups-yet = Nenhum backup deste perfil ainda.
kind-complete = Completo
kind-incremental = Incremental

# Status bar
ready = Pronto
cancel = Cancelar
undo = Desfazer
phase-starting = Iniciando…
phase-opening = Abrindo o destino…
phase-scanning = Procurando alterações…
phase-copying = Copiando…
phase-merging = Mesclando backups antigos…
phase-restoring = Restaurando…
phase-verifying = Verificando…
progress-of = { $done } de { $total }
skipped-count = { $count ->
    [one] { $count } arquivo ignorado (em uso)
   *[other] { $count } arquivos ignorados (em uso)
}
profile-removed = Perfil "{ $name }" excluído.
run-finished = Backup de "{ $name }" feito: { $new } novos, { $changed } alterados, { $deleted } excluídos, { $added } copiados.
merge-finished = "{ $name }" mesclado: de { $before } para { $after } backups, { $freed } liberados.
restore-finished = { $count } arquivos restaurados.
verify-finished = { $count } arquivos verificados, { $problems } problemas.
run-destination-missing = Destino não encontrado: { $location }. Conecte o disco e tente novamente.
run-busy = Outro backup ou mesclagem deste perfil está em andamento; tente de novo quando terminar.
run-failed = A operação falhou: { $detail }
run-no-space = Não há espaço livre suficiente no destino: são necessários { $needed } e há { $available } livres. Libere espaço, ou mescle ou exclua backups antigos, e execute de novo: os arquivos já copiados são mantidos e não são copiados outra vez.
restore-no-space = Não há espaço livre suficiente onde os arquivos são restaurados: são necessários { $needed } e há { $available } livres. Nada foi gravado: libere espaço, ou restaure menos arquivos ou em outro disco.
backup-resumed = { $count ->
    [one] { $count } arquivo aproveitado do backup interrompido
   *[other] { $count } arquivos aproveitados do backup interrompido
}
backup-as-changes = { $count ->
    [one] { $count } arquivo salvo só como alterações
   *[other] { $count } arquivos salvos só como alterações
}
run-cancelled = Cancelado. Nada foi alterado nos backups; um backup interrompido continua de onde parou na próxima vez.

# Errors
settings-unreadable = Não é possível ler o arquivo de configurações, portanto as alterações não serão salvas: { $detail }
save-failed = Não foi possível salvar as configurações: { $detail }
schedule-failed = Não foi possível agendar a execução automática: { $detail }
start-failed = Não foi possível iniciar a operação: { $detail }
unexpected-stop = A operação parou inesperadamente.

# Restore browser
dates = Datas dos backups
archive-tooltip = { $files } arquivos, { $stored } salvos neste backup ({ $size })
search-hint = Pesquisar um arquivo pelo nome em todos os backups…
no-results = Nenhum arquivo com esse nome.
results = { $count ->
    [one] { $count } arquivo encontrado
   *[other] { $count } arquivos encontrados
}
close = Fechar
versions-hint = Cada versão é um conteúdo diferente do arquivo, da mais recente para a mais antiga.
version-modified = Modificado em { $date }
version-detail = Salvo pelo backup de { $backup } · { $size }
restore-to = Restaurar em:
target-beside = Ao lado do original, com a data
target-beside-hint = relatorio.docx é restaurado como "relatorio (2026-11-02 21.30).docx": o arquivo atual não é alterado.
target-original = Local original
target-folder = Outra pasta
replace-existing = Substituir arquivos que já existem
replace-warning = Os arquivos atuais serão substituídos pela cópia do backup.
tick-files-hint = Marque pelo menos um arquivo ou pasta.
choose-folder-hint = Escolha a pasta onde restaurar.
restore-selected = Restaurar
clear-selection = Limpar seleção
tick-folder = Restaurar esta pasta e todo o seu conteúdo
tick-file = Restaurar este arquivo
backups-restore-files = Restaurar arquivos
backups-restore-files-hint = Todos os arquivos já salvos: marque o que quer recuperar, escolha as versões ou a data e para onde vai.
backups-manage = Gerenciar backups
backups-manage-hint = A lista de datas dos backups: mescle vários em um ou exclua um.
restore-tree-count = { $files ->
        [one] { $files } arquivo
       *[other] { $files } arquivos
    } nos backups · excluídos na origem: { $deleted }
restore-tree-hint = Marque arquivos ou pastas para restaurá-los; a pesquisa encontra um arquivo pelo nome. Arquivos marcados com { $deleted-icon } excluído não estão mais na origem e ainda podem ser restaurados.
deleted-label = excluído
deleted-since = Não está no backup mais recente: excluído na origem.
tree-deleted-file = Não está no backup mais recente: excluído na origem. Marque-o para restaurar uma de suas versões.
tree-deleted-folder = Não está no backup mais recente: excluída na origem. Marque-a para restaurá-la como estava em uma data.
remove-from-selection = Remover da seleção
tick-in-folder = Restaurado com a pasta marcada { $folder }. Desmarque para deixá-lo de fora.
tick-not-in-folder-date = Não está no backup com que volta a pasta marcada { $folder }: não é restaurado. Desmarque essa pasta para escolhê-lo sozinho.
tick-partial = Há algo marcado dentro, ou deixado de fora da pasta marcada. Clique para marcar a pasta inteira.
tick-full = Tudo o que ela contém está marcado um a um. Clique para desmarcar tudo.
tick-left-out = Deixado de fora da pasta marcada { $folder }: não é restaurado. Marque para restaurá-lo com a pasta.
folder-left-out = { $count ->
    [one] { $count } item desmarcado dentro: não é restaurado.
   *[other] { $count } itens desmarcados dentro: não são restaurados.
}
card-show-options = Clique para ver e mudar as versões, a data e o destino.
card-summary-file = { $versions ->
    [one] 1 versão → { $dest }
   *[other] { $versions } versões → { $dest }
}
card-summary-folder = Como estava em { $date } → { $dest }
selection-empty = Ainda não há nada para restaurar. Marque arquivos ou pastas na árvore: cada um aparece aqui com suas versões ou sua data, e com o destino.
folder-as-of = Como estava em:
folder-as-of-hint = A pasta volta inteira, com tudo o que continha naquela data.
folder-size = { $size } naquela data
dest-overwrite = Substituir o original
dest-folder-missing = outra pasta, ainda não escolhida
restore-to-hint = Para onde vai tudo o que está marcado.
restore-conflict = Outra versão substitui o mesmo original: deixe "Substituir o original" em apenas uma versão.
restore-conflict-hint = Duas versões substituiriam o mesmo arquivo original: altere o destino de uma delas.
restore-wait-versions = Lendo as versões dos arquivos marcados…
restore-summary = { $files ->
        [one] { $files } arquivo
       *[other] { $files } arquivos
    }, { $versions ->
        [one] { $versions } versão
       *[other] { $versions } versões
    }, { $folders ->
        [one] { $folders } pasta
       *[other] { $folders } pastas
    } — { $size }

# Manual merge
merge-choose = Escolher backups para mesclar…
merge-choose-hint = Marque o primeiro e o último backup: tudo o que está entre eles vira um único backup.
merge-tick = Marcar como extremo do intervalo a mesclar
merge-tick-hint = Marque o backup mais antigo e o mais recente a mesclar; os intermediários são incluídos.
merge-summary = { $count } backups viram um só, com data de { $date }.
merge-complete-warning = O intervalo inclui o primeiro backup completo: ele é substituído por um novo backup completo com a data do mais recente escolhido, e as datas anteriores não poderão mais ser restauradas.
merge-complete-ack = Entendo
merge-two-hint = Marque pelo menos dois backups.
merge-ack-hint = Confirme que você entendeu o aviso.
merge-selected = Mesclar

# Activity tab
tab-activity = Atividade
show-details = Detalhes
cancel-hint = Parar agora: os backups ficam como estavam antes desta execução. O próximo backup reaproveita os arquivos já copiados.
activity-backup-running = Fazendo backup de "{ $name }"
activity-merge-running = Mesclando os backups de "{ $name }"
activity-restore-running = Restaurando de "{ $name }"
activity-preparing = Preparando…
activity-file = Arquivo
activity-folder = Pasta
activity-files = Arquivos
activity-speed = Velocidade
activity-elapsed = Decorrido
activity-remaining = Restante
count-of = { $done } de { $total }
files-found = { $count } encontrados até agora
about = cerca de { $time }
activity-took = Levou { $time }.
activity-backup-summary = Concluído: { $new } novos, { $changed } alterados, { $deleted } excluídos, { $unchanged } inalterados; { $copied } copiados.
activity-merge-summary = Concluído: de { $before } para { $after } backups, { $freed } liberados.
activity-restore-summary = Concluído: { $restored } arquivos restaurados, { $kept } já presentes e mantidos.
skipped-hint = Não foi possível ler estes arquivos, geralmente porque outro programa os mantém abertos. A versão anterior deles, se houver, continua nos backups.
skipped-more = …e mais { $count } (veja o registro).

# Exclusions and format lock
no-exclusions = Nada excluído: todos os arquivos das pastas entram no backup.
remove-exclusion = Voltar a incluir no backup
exclusion-folder = Pasta e todo o seu conteúdo
exclusion-file = Arquivo
exclusion-type = Todos os arquivos deste tipo
exclusion-pattern = Padrão
exclude-folder = Excluir pasta…
exclude-file = Excluir arquivo…
exclude-type-hint = .tmp
exclude-type = Excluir tipo
common-types = Comuns:
advanced = Avançado
exclude-pattern = Excluir padrões
exclude-pattern-add = Adicionar
container-locked = O destino já contém backups neste formato: ele não pode mudar. Para usar o outro, escolha um destino vazio.
container-unreachable = Não é possível acessar o destino, portanto o formato não pode ser alterado agora.
container-checking = Verificando o destino…
activity-idle-title = Nada em andamento
activity-idle-body = Backups, mesclagens e restaurações mostram o andamento aqui.

# Backup kinds and deletion
menu-run-incremental = Backup incremental agora
menu-run-complete = Backup completo agora
run-now = Executar agora:
run-incremental = Incremental
run-incremental-hint = Copia só os arquivos novos e alterados; rápido, e a escolha habitual.
run-complete = Completo
run-complete-hint = Copia todos os arquivos e inicia uma nova cadeia: os backups seguintes não dependem mais dos anteriores. Leva mais tempo e ocupa mais espaço.
run-merge = Mesclar
run-first = Fazer o primeiro backup
run-incremental-button = Fazer backup incremental
run-complete-button = Fazer backup completo
full-backup = Backup completo:
full-never = Só o primeiro
full-every-backups = A cada
full-every-days = Quando o último tiver mais de
full-backups-unit = backups
full-days-unit = dias
full-backup-hint = Vale para os backups automáticos; um backup completo inicia uma nova cadeia, o que torna a restauração independente dos backups mais antigos.
run-finished-complete = Backup completo de "{ $name }" feito: { $new } novos, { $changed } alterados, { $deleted } excluídos, { $added } copiados.
activity-complete-summary = Backup completo concluído: { $new } novos, { $changed } alterados, { $deleted } excluídos, { $unchanged } inalterados; { $copied } copiados.
activity-delete-running = Excluindo um backup de "{ $name }"
activity-delete-summary = Concluído: de { $before } para { $after } backups, { $freed } liberados.
delete-finished = Um backup de "{ $name }" foi excluído.
delete-backup = Excluir este backup…
delete-backup-hint = Remove a data selecionada; os backups seguintes continuam completos.
delete-backup-confirm = Excluir backup
delete-initial-hint = O primeiro backup completo só pode ser excluído quando é o único: os outros dependem dele.
delete-complete-hint = Um backup completo inicia uma cadeia da qual dependem os backups seguintes: ele não pode ser excluído. Em vez disso, mescle-o com os seguintes.
delete-only-warning = O backup de { $date } é o único: o destino ficará vazio e o próximo backup será completo. Isso não pode ser desfeito.
delete-newest-warning = O backup de { $date } será excluído, com as versões de arquivos salvas só nele. Isso não pode ser desfeito.
delete-middle-warning = O backup de { $date } será excluído. Os arquivos de que os backups seguintes precisam passam para o próximo backup; as versões salvas só neste são perdidas. Isso não pode ser desfeito.

# Encryption
encrypt = Criptografar os backups (AES-256, incluindo nomes de arquivos)
encrypt-hint = Cada arquivo é um zip AES-256 com nome oculto; sem a senha, ninguém consegue ler o conteúdo nem ver quais arquivos estão salvos. A senha fica guardada no Gerenciador de Credenciais do Windows.
encrypt-needs-zip = A criptografia requer o formato "Um zip por arquivo".
password = Senha
password-again = Repita a senha
password-set = Escolha a senha
password-reenter = Digite a senha destes backups
password-saved = Senha salva no Gerenciador de Credenciais do Windows.
password-missing = Nenhuma senha salva neste computador: digite-a para fazer backups e navegar por eles.
password-loss-warning = Sem esta senha, ou a chave de recuperação mostrada após o primeiro backup, os backups não podem ser abertos e ninguém consegue recuperá-los. Guarde a senha em um lugar seguro.
password-save = Salvar senha
password-checking = Verificando a senha…
password-empty-hint = Digite a senha.
password-short-hint = Use pelo menos { $count } caracteres.
password-mismatch-hint = As duas senhas são diferentes.
password-rule-length = Pelo menos { $count } caracteres (agora { $typed })
password-rule-match = As duas senhas coincidem
password-rule-variety = Pelo menos { $kinds } tipos de caracteres entre minúsculas, maiúsculas, dígitos e símbolos (dispensável a partir de { $passphrase } caracteres)
password-rule-common = Não é uma senha comum, como "Password2024" ou "qwerty"
password-rule-runs = Nenhuma sequência de { $count } caracteres repetidos ou consecutivos, como "aaaa", "1234" ou "abcd"
password-rule-name = Não contém o nome do perfil
password-rule-strength = Difícil o bastante de adivinhar: força mínima { $level }
password-rule-tip = Qualquer caractere vale, inclusive espaços: uma frase de quatro ou cinco palavras é fácil de lembrar e difícil de adivinhar e, a partir de { $passphrase } caracteres, dispensa dígitos, maiúsculas ou símbolos.
password-strength = Força: { $level }
password-strength-weak = Fraca
password-strength-fair = Razoável
password-strength-strong = Forte
password-strength-very-strong = Muito forte
password-variety-hint = Misture minúsculas, maiúsculas, dígitos e símbolos, ou escreva uma frase de pelo menos { $passphrase } caracteres.
password-common-hint = Esta é uma das senhas mais usadas: escolha outra.
password-runs-hint = Evite sequências como "aaaa" ou "1234".
password-name-hint = Não coloque o nome do perfil na senha.
password-weak-hint = Fácil demais de adivinhar: deixe-a mais longa, por exemplo com mais algumas palavras.
password-first-hint = Digite acima a senha dos backups criptografados (duas vezes para novos backups, e marque "Entendo"): o botão a salva e inicia o backup.
password-wrong = Esta senha não abre os backups do destino.
password-unreachable = Não foi possível verificar a senha
run-password-needed = Os backups estão criptografados e não há senha salva neste computador: digite-a nas configurações do perfil.
run-wrong-password = A senha salva não abre os backups: digite-a novamente nas configurações do perfil.

# Operations log
log-runs = Operações
log-only-profile = Só este perfil
log-running = Em andamento
log-empty = Nenhuma operação ainda.
log-pick = Escolha uma operação à esquerda.
log-title = Registro
log-no-lines = Nada registrado ainda.
log-scheduled = automática
log-manual = iniciada manualmente
log-started = Iniciada { $time }
log-backup = backup { $name }
log-unfinished = Não terminou: foi interrompida ou ainda está em andamento.
log-skipped = Ignorado { $path }: { $reason }
log-op-backup = Backup
log-op-incremental = Backup incremental
log-op-complete = Backup completo
log-op-merge = Mesclagem
log-op-delete = Exclusão de backup
log-op-restore = Restauração
log-open-full = Abrir registro completo
log-open-full-hint = Grava o registro completo desta operação em um arquivo de texto e o abre no seu editor de texto: cada etapa e depois cada arquivo com que ela trabalhou e o que fez com ele, incluindo os ignorados.
log-copy = Copiar registro
log-copy-hint = Copia o registro completo desta operação como texto, para colá-lo em um e-mail ou em um relato de problema.
log-copied = Registro copiado.
log-open-program = Registro do programa
log-open-program-hint = Abre o archeion.log, o registro técnico do programa: útil para investigar um problema ou relatá-lo.
log-open-folder = Pasta de registros
log-open-folder-hint = Abre a pasta dos registros das operações no Explorador de Arquivos.
log-program-missing = O registro do programa ainda não existe.
log-unreadable = Não é possível ler o arquivo de registro.
log-report-failed = Não é possível mostrar o registro completo: { $detail }
log-level-info = Info
log-level-done = OK
log-level-warning = Aviso
log-level-error = Erro
log-preparing = Preparando o registro completo…
log-files-title = Arquivos
log-files-missing = A lista de arquivos não foi registrada para esta operação: ela foi feita por uma versão anterior do Archeion.
log-files-none = Nenhum arquivo processado.
log-file-new-chunks = salvo como alterações: { $size } de blocos novos gravados
log-file-resumed = já copiado pelo backup interrompido, mantido
log-count-new = Novos: { $count }
log-count-changed = Alterados: { $count }
log-count-as-changes = Salvos como alterações: { $count }
log-count-copied-again = Copiados de novo: { $count }
log-count-unchanged = Inalterados, não listados: { $count }
log-count-deleted = Excluídos: { $count }
log-count-moved = Movidos: { $count }
log-count-dropped = Versões removidas: { $count }
log-count-restored = Restaurados: { $count }
log-count-kept = Mantidos: { $count }
log-count-checked = Relidos intactos: { $count }
log-count-checked-unlisted = Relidos intactos, não listados: { $count }
log-count-damaged = Problemas: { $count }
log-count-skipped = Ignorados: { $count }

# Password change and recovery key
password-change = Alterar a senha
password-new = Nova senha
password-change-button = Alterar senha
recovery-later = A chave de recuperação estará disponível após o primeiro backup.
recovery-show = Mostrar a chave de recuperação…
recovery-title = Chave de recuperação
recovery-hint = Com esta chave, o 7-Zip abre todos os arquivos e manifestos criptografados, mesmo sem o Archeion e sem a senha. O manifesto de cada backup informa onde cada arquivo está guardado.
recovery-warning = Quem tiver esta chave pode ler todos os backups. Imprima-a ou anote-a e guarde-a offline; ela não muda quando você altera a senha.
recovery-copy = Copiar
recovery-copy-hint = Copia a chave sem espaços, como o 7-Zip exige.
recovery-hide = Ocultar
password-current = Senha atual
password-current-hint = Digite a senha atual.
recovery-ask = Para mostrar a chave de recuperação, digite a senha atual destes backups.
recovery-show-button = Mostrar

# Integrity check
verify-section = Verificações
verify-hint = A verificação completa relê cada arquivo guardado de cada backup e o compara com sua impressão digital; o teste de restauração relê aleatoriamente alguns arquivos do backup mais recente. As verificações pendentes são executadas após um backup agendado.
verify-full-every = Verificação completa a cada
verify-sample-every = Teste de restauração a cada
verify-sample-files = em arquivos:
verify-zero-hint = 0 dias = nunca automaticamente.
verify-full-now = Verificar backups agora
verify-full-hint = Relê e verifica cada arquivo guardado; em um destino grande, leva o mesmo tempo que ler todos os backups.
verify-sample-now = Teste de restauração agora
verify-sample-hint = Relê aleatoriamente alguns arquivos do backup mais recente, como faria uma restauração, sem gravá-los em lugar nenhum.
verify-no-backups = Ainda não há backups para verificar.
verify-problems = Problemas encontrados
verify-problems-hint = Os backups listados podem não ser restaurados corretamente. Faça um backup completo agora e verifique o disco de destino.
activity-verify-running = Verificando os backups de "{ $name }"
log-op-verify = Verificação de integridade
log-op-restore-test = Teste de restauração

# Automatic backup: schedules, events, notifications
schedule-days = Nos dias escolhidos
schedule-hours = A cada poucas horas
schedule-monthly = Uma vez por mês
schedule-all-days = Todos os dias
schedule-add-time = + Adicionar um horário
schedule-remove-time = Remover este horário
schedule-every = A cada
hours-unit = horas
schedule-from = das
schedule-until = até
schedule-hours-hint = A última execução começa antes do horário final; o mesmo horário de início e fim significa o dia todo.
schedule-month-day = No dia
schedule-month-hint = Só os dias de 1 a 28, para que todo mês tenha esse dia.
events-title = Fazer backup também:
event-logon = Alguns minutos depois que eu entrar no Windows
event-idle = Quando o computador estiver ocioso por 10 minutos
event-drive = Quando uma unidade for conectada
event-drive-hint = Útil com um disco externo como destino: o backup começa quando ele é conectado, e nada acontece quando outra unidade é conectada.
event-min-hours = No máximo a cada
event-hint = Um backup iniciado por um destes eventos é ignorado quando o último backup é mais recente ou quando o destino não está conectado.
alerts = Notificações
alert-on-failure = Avisar quando um backup automático falhar
alert-stale = Avisar quando não houver backup há
alert-hint = Notificações do Windows; 0 dias = sem aviso. A verificação é feita todos os dias e depois de entrar no Windows.
alert-manual-hint = Só para backups automáticos: escolha um agendamento em "Backup automático".
alert-failed-title = Archeion: o backup de "{ $name }" falhou
alert-stale-title = Archeion: "{ $name }" sem backup
alert-stale-body = Nenhum backup há { $days } dias (o último em { $when }). Verifique se o destino está conectado ou inicie um backup pelo Archeion.
closing-after-run = Uma operação está em andamento: o Archeion será fechado sozinho quando ela terminar.
closing-cancel = Não fechar

# Notification area and Preferences window
menu-preferences = Preferências…
preferences-title = Preferências
preferences-window = Janela
language = Idioma
language-system = Idioma do sistema
language-tray-restart = O menu do ícone da área de notificação muda de idioma na próxima vez que o Archeion for iniciado.
quit-hint = Fecha o Archeion; os backups agendados continuam sendo executados pelo Agendador de Tarefas do Windows.
tray-open = Abrir o Archeion
tray-quit = Sair
tray-running = operação em andamento
option-tray = Continuar na área de notificação quando a janela for fechada
option-tray-hint = Fechar a janela a oculta ao lado do relógio: o Archeion continua mesclando backups antigos e concluindo a operação em andamento. Clique no ícone para abri-la de novo ou escolha Sair.
option-tray-unavailable = O Windows não forneceu um ícone na área de notificação.
option-autostart = Iniciar o Archeion com o Windows
option-autostart-hint = Quando você entra no Windows, o Archeion é iniciado oculto na área de notificação. Você também pode desativar isso no Gerenciador de Tarefas, em Aplicativos de inicialização.
autostart-failed = Não foi possível alterar a inicialização com o Windows: { $detail }

# Help menu: user guide and About
menu-documentation = Documentação
menu-about = Sobre o Archeion
about-version = Versão { $version }
about-name = O nome vem do grego antigo ἀρχεῖον, "arquivo".
about-author = Autor: MR
about-license = Licença: GPL-3.0
about-repository = Código-fonte no GitHub
about-logs-folder = Pasta de registros
about-open-folder = Abrir pasta
about-folder-missing = A pasta ainda não existe.
doc-title = Documentação do Archeion
doc-intro = O Archeion mantém cópias das suas pastas em outro disco, em um compartilhamento de rede ou em qualquer outra pasta, e devolve qualquer arquivo como ele estava em uma data passada. Abra uma seção para lê-la.
doc-what-title = O que o Archeion faz
doc-what-backups = O primeiro backup de um perfil é completo: copia todos os arquivos. Os seguintes são incrementais: copiam só os arquivos novos e alterados, por isso são rápidos e pequenos, e mesmo assim cada data pode ser restaurada por inteiro. Em "Backup completo", nas configurações do perfil, você também pode pedir um novo backup completo a cada certo número de backups, ou quando o último tiver mais de alguns dias; um backup completo inicia uma nova cadeia que não depende mais dos backups mais antigos.
doc-what-storage = Cada arquivo é guardado separadamente, nas mesmas pastas do original: como um zip desse arquivo ("Um zip por arquivo": relatorio.docx vira relatorio.docx.zip), como uma cópia simples ou como um zip criptografado com AES-256. Os backups podem ser lidos sem o Archeion: o Explorador de Arquivos e o 7-Zip abrem os zips, e o 7-Zip abre os criptografados com a chave de recuperação.
doc-what-safe = Um backup interrompido (computador desligado, disco removido, Cancelar) deixa os backups existentes como estavam; o próximo backup continua de onde parou e não copia de novo os arquivos já copiados.
doc-profiles-title = Perfis
doc-profiles-body = Um perfil define o que incluir no backup, onde e quando. Crie um em Arquivo > Novo perfil (Ctrl+N) e preencha a guia Configurações: um nome, as pastas para backup e o destino, que pode ser uma pasta, um disco externo ou um compartilhamento de rede. Use cada pasta de destino para um único perfil.
doc-profiles-exclusions = As exclusões deixam de fora o que você não precisa: uma pasta e todo o seu conteúdo, um arquivo ou todos os arquivos de um tipo, como .tmp. Em "Excluir padrões" você pode adicionar padrões, comparados com o caminho completo sem diferenciar maiúsculas: * é qualquer texto em um nome, ** quaisquer pastas, ? um caractere. Por exemplo **/node_modules ou **/cache/**.
doc-profiles-format = O formato de armazenamento (um zip por arquivo ou cópias simples) e a criptografia devem ser escolhidos antes do primeiro backup: depois que o destino contém backups, eles não podem mudar. Para trocar, escolha um destino vazio.
doc-profiles-delete = Excluir perfil pede confirmação e depois remove o perfil e suas tarefas agendadas; os backups permanecem no destino. Logo em seguida, Desfazer na barra de status traz o perfil de volta. Um perfil que use essa pasta mais tarde pode assumi-los com "Usar estes backups neste perfil".
doc-auto-title = Backups automáticos
doc-auto-schedule = Em "Backup automático", escolha quando os backups são feitos sozinhos: nos dias escolhidos, em um ou mais horários; a cada poucas horas entre dois horários; ou uma vez por mês, em um dia de 1 a 28. O Archeion registra o agendamento no Agendador de Tarefas do Windows, então os backups são feitos mesmo quando o Archeion não está aberto.
doc-auto-missed = Se o computador estiver desligado no horário agendado, o backup é feito assim que ele for ligado de novo.
doc-auto-alerts = Em "Notificações", o Archeion pode mostrar uma notificação do Windows quando um backup automático falha, e um aviso quando um perfil está sem backup há um certo número de dias (verificado todos os dias e depois que você entra no Windows).
doc-auto-now = Você sempre pode iniciar um backup por conta própria: Fazer backup agora nas configurações do perfil, ou Backup > Backup incremental agora (Ctrl+B) e Backup completo agora.
doc-merge-title = Mesclagem de backups antigos
doc-merge-body = Os backups antigos não são excluídos, mas mesclados ao próximo backup que é mantido: cada data mantida continua completa, e o espaço das versões que não são mais necessárias é liberado. A mesclagem é automática: acontece enquanto o Archeion está em execução, nunca enquanto um backup ou outra operação está usando o destino.
doc-merge-rules = As regras em "Mesclar backups antigos" definem quais backups manter, e um backup é mantido se alguma regra o selecionar: todos os backups dos últimos dias, os mais recentes, um por dia, um por semana, um por mês e um por ano, cada um pela quantidade definida. Com todas as regras em 0, todos os backups são mantidos e nada é mesclado.
doc-merge-first = O primeiro backup completo nunca é mesclado automaticamente.
doc-merge-manual = Em Backups > Gerenciar backups, "Escolher backups para mesclar…" mescla manualmente: marque o backup mais antigo e o mais recente, e tudo o que está entre eles vira um único backup. Se o intervalo incluir o primeiro backup completo, o Archeion avisa antes, porque as datas anteriores não poderão mais ser restauradas.
doc-merge-delete = "Excluir este backup…" em Backups > Gerenciar backups remove uma data; os backups seguintes continuam completos. Um backup completo do qual dependem backups posteriores não pode ser excluído (mescle-o em vez disso). Uma exclusão não pode ser desfeita.
doc-encrypt-title = Criptografia
doc-encrypt-body = Com "Criptografar os backups", cada arquivo vira um zip AES-256 com nome oculto: sem a senha, ninguém consegue ler o conteúdo nem ver quais arquivos estão salvos. A criptografia requer o formato "Um zip por arquivo" e deve ser ativada antes do primeiro backup; depois, não pode ser alterada.
doc-encrypt-password = A senha é salva no Gerenciador de Credenciais do Windows, então os backups automáticos não exigem digitação. Você pode alterá-la depois nas configurações do perfil, digitando a atual. Em outro computador, digite-a uma vez nas configurações do perfil.
doc-encrypt-recovery = Depois do primeiro backup, "Mostrar a chave de recuperação…" mostra uma chave com a qual o 7-Zip abre todos os arquivos criptografados, mesmo sem o Archeion e sem a senha. Imprima-a ou anote-a e guarde-a offline: sem a senha e sem a chave, ninguém consegue abrir os backups. A chave não muda quando você altera a senha. O Archeion pede a senha atual antes de mostrá-la.
doc-large-title = Arquivos grandes
doc-large-body = Um arquivo grande que muda pouco (uma caixa de correio, o disco de uma máquina virtual, um banco de dados) não é copiado de novo por inteiro: é salvo em partes de cerca de 1 MB, e cada backup guarda só as partes novas. Partes idênticas são guardadas uma só vez. Vale para arquivos de pelo menos o tamanho escolhido na seção Formato do perfil (500 MB se não for alterado; pode ser desativado) e para todos os arquivos de um destino criptografado.
doc-large-complete = Os backups completos continuam guardando cada arquivo inteiro, como zip, cópia simples ou zip criptografado, então podem ser restaurados só com o 7-Zip; apenas os backups incrementais seguintes contêm partes. A restauração junta as partes de novo e confere cada uma; a verificação de integridade lê todas as partes.
doc-large-upgrade = Os backups feitos por versões anteriores ficam como estão e continuam legíveis. As partes começam com o próximo backup completo: num destino assim, o próximo backup automático é completo.
doc-restore-title = Restauração de arquivos
doc-restore-body = Abra a guia Backups do perfil (ou Backup > Restaurar arquivos…): "Restaurar arquivos" mostra uma única árvore com todos os arquivos e pastas que os backups já salvaram; arquivos que não estão mais na origem aparecem marcados com { $deleted-icon } excluído e ainda podem ser restaurados. Marque arquivos ou pastas na árvore ou nos resultados da pesquisa (a caixa de pesquisa encontra um arquivo pelo nome). Cada item marcado aparece à direita; { $remove-icon } o remove.
doc-restore-versions = Um arquivo marcado lista suas versões, da mais recente para a mais antiga, cada uma com um conteúdo diferente: a mais recente vem marcada e você pode marcar outras. Uma pasta marcada volta inteira, como estava na data escolhida em "Como estava em" (a mais recente, a menos que você a altere).
doc-restore-where = No alto do painel direito escolha para onde vai tudo: ao lado do original, com a data no nome (relatorio.docx volta como "relatorio (2026-11-02 21.30).docx" e o arquivo atual não é alterado), o local original ou outra pasta; os arquivos existentes só são substituídos se você marcar "Substituir arquivos que já existem". Várias versões de um arquivo enviadas para a mesma pasta recebem cada uma sua data no nome; apenas uma versão de um arquivo pode substituir o original. Restaurar traz tudo de volta em uma única operação. "Limpar seleção", ao lado, desmarca tudo.
doc-verify-title = Verificação de integridade e teste de restauração
doc-verify-body = A verificação completa relê cada arquivo guardado de cada backup e o compara com sua impressão digital; o teste de restauração relê aleatoriamente alguns arquivos do backup mais recente, como faria uma restauração, sem gravá-los em lugar nenhum. Inicie-os na seção Verificações das configurações do perfil (ou em Backup > Verificar backups agora), ou faça com que sejam executados a cada poucos dias após um backup agendado. Se forem encontrados problemas, faça um backup completo e verifique o disco de destino.
doc-activity-title = Guia Atividade
doc-activity-body = A guia Atividade mostra o andamento do backup, da mesclagem, da restauração ou da verificação em execução, com um botão Cancelar, e o registro de cada operação anterior, automática ou iniciada manualmente: o que ela fez e quais arquivos foram ignorados porque outro programa os mantinha abertos. Acima das etapas de uma operação, Abrir registro completo mostra todo o registro dela como um arquivo de texto no seu editor: as etapas e depois cada arquivo com que ela trabalhou, com a hora, o que aconteceu com ele (novo, alterado, salvo como alterações, copiado de novo, excluído, movido, versão removida, restaurado, mantido, ignorado, problema), o tamanho e o caminho completo; os arquivos inalterados de um backup incremental e os intactos de uma verificação completa são apenas contados. Copiar registro copia o mesmo texto; Registro do programa abre o registro técnico do Archeion, para problemas mais complexos.
doc-window-title = Fechar a janela
doc-window-body = Fechar a janela não encerra o Archeion: ele vai para a área de notificação, ao lado do relógio, e continua trabalhando, de modo que os backups antigos continuam sendo mesclados; abra a janela de novo pelo ícone. O Archeion também é iniciado com o Windows, oculto ali. Arquivo > Sair, ou Sair no menu do ícone, fecha-o de verdade. As duas coisas podem ser desativadas em Arquivo > Preferências, onde você também escolhe o idioma. Os backups agendados são executados de qualquer forma, mesmo quando o Archeion não está em execução.
doc-cli-title = Linha de comando
doc-cli-body = archeion.exe sem argumentos abre a janela. Os comandos abaixo executam uma operação sem a janela, como fazem as tarefas agendadas. O id do perfil é o id do perfil em settings.toml.
doc-cli-run = Faz o backup do perfil (--kind complete para um backup completo).
doc-cli-merge = Mescla os backups antigos segundo as regras do perfil.
doc-cli-verify = Verifica os backups (--sample N: teste de restauração de N arquivos).
doc-cli-delete = Exclui um backup.
doc-cli-restore = Restaura os arquivos listados em um arquivo de solicitação gravado pela janela.
doc-cli-watch = Avisa sobre perfis sem backup recente; o Agendador de Tarefas o executa todos os dias.
doc-cli-extract = Restaura a partir de uma pasta de backups sozinha, sem configurações nem perfil: por exemplo com o Archeion portátil em outro computador. Os arquivos vão para baixo da pasta de destino com o caminho completo.
doc-cli-extract-options = Opções de extract: --archive <nome> escolhe o backup (o mais recente se omitido); --path <arquivo ou pasta> restaura só isso, indicado pelo caminho original; --overwrite substitui arquivos existentes. Para backups criptografados, informe a senha ou a chave de recuperação com --stdin (primeira linha da entrada padrão), com a variável ARCHEION_PASSWORD ou ARCHEION_RECOVERY_KEY, ou com --password ou --recovery-key. O progresso também é gravado em archeion-extract.log na pasta de destino. No Prompt de Comando, execute como start /wait archeion extract … para que o prompt espere.
doc-cli-exit = Códigos de saída: 0 sucesso, 1 falha, 2 destino não encontrado, 3 ocupado (outra operação está usando o destino), 4 senha ausente ou incorreta, 5 espaço livre insuficiente.
doc-data-title = Onde ficam os dados e os registros
doc-data-body = As configurações ficam na pasta %APPDATA%\Archeion\config (settings.toml), e o registro de cada operação fica na subpasta logs. Uma cópia portátil (com um arquivo portable.txt ao lado do archeion.exe) mantém tudo em uma pasta data ao lado do executável. Ajuda > Sobre o Archeion abre essas pastas.
doc-data-backups = Os backups em si ficam só no destino: uma pasta por backup, com o nome da data e hora, contendo os arquivos guardados e uma lista de todos os arquivos naquele momento.

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
    [one] { $count } arquivo encontrado
   *[other] { $count } arquivos encontrados
}
step-folders = { $count ->
    [one] { $count } pasta lida
   *[other] { $count } pastas lidas
}
step-new = { $count } novos
step-changed = { $count } alterados
step-unchanged = { $count } inalterados
step-copied-again = { $count } inalterados copiados de novo
step-moved = { $count ->
    [one] { $count } arquivo movido
   *[other] { $count } arquivos movidos
}
step-restored = { $count } restaurados
step-kept = { $count } já presentes, mantidos
step-checked = { $count } verificados
step-damaged = { $count ->
    [one] { $count } problema
   *[other] { $count } problemas
}
step-skipped = { $count } ignorados
merge-group = { $count } backups, de { $first } a { $last }, em um só
merge-group-of = Grupo { $index } de { $of }: { $count } backups, de { $first } a { $last }, em um só
recent-title = Últimos arquivos
recent-new = Novo
recent-changed = Alterado
recent-copied-again = Copiado de novo
recent-moved = Movido
recent-restored = Restaurado
recent-kept = Mantido
recent-checked = OK
recent-damaged = Problema
recent-skipped = Ignorado
recent-deleted = Excluído
recent-dropped = Versão removida

# Profile files (File menu)
import-profile = Importar perfil…
export-profile = Exportar perfil…
import-profile-hint = Adiciona um perfil de um arquivo exportado pelo Archeion: as regras dele, não a senha.
export-profile-hint = Salva todas as regras deste perfil em um arquivo, para guardá-lo ou levá-lo a outro computador. A senha não é salva nele.
profile-file-type = Perfil do Archeion
import-failed-read = Não foi possível ler o arquivo: { $detail }
import-failed-format = { $file } não é um perfil do Archeion.
import-failed-newer = { $file } foi criado por uma versão mais recente do Archeion: atualize o Archeion para importá-lo.
export-failed = Não foi possível salvar o perfil: { $detail }
doc-profiles-files = Arquivo > Exportar perfil… salva todas as regras do perfil selecionado (pastas, exclusões, destino, formato, agendamento, mesclagem, verificações, notificações) em um arquivo .archeion-profile, para guardá-lo ou levá-lo a outro computador; Arquivo > Importar perfil… o adiciona de novo. A senha nunca fica no arquivo: um perfil criptografado a pede após a importação. Em outro computador, o perfil importado encontra seus backups no destino; importado duas vezes no mesmo, vira uma cópia com "(2)" no nome.
