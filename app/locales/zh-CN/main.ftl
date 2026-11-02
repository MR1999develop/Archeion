# Menus
menu-file = 文件
menu-backup = 备份
menu-help = 帮助
new-profile = 新建配置
quit = 退出
restore = 还原文件…
data-folder = 数据文件夹

# Profile list and empty state
profiles = 配置
unnamed = （未命名）
new-profile-name = 新备份
empty-title = 还没有备份
empty-body = 创建一个配置：选择要备份的文件夹、保存位置和备份时间。
tab-settings = 设置
tab-backups = 备份

# Profile form
profile-name = 名称
folders = 要备份的文件夹
no-folders = 还没有文件夹：请至少添加一个。
add-folder = 添加文件夹…
remove-folder = 从备份中移除此文件夹
exclusions = 排除项
exclusions-hint = 模式匹配完整路径，不区分大小写：* 表示名称中的任意文本，** 表示任意层文件夹，? 表示一个字符。例如 **/node_modules、**/cache/**、**/~$*。
destination = 目标位置
dest-local = 文件夹或外部驱动器
dest-network = 网络共享
dest-hint = 请使用仅供此配置使用的文件夹。除非启用了加密（见下文），否则任何能打开该文件夹的人都能读取备份。
dest-free = 目标位置可用空间：{ $free }
dest-foreign-profile = 此文件夹存放的是配置“{ $name }”的备份：请为此配置选择其他文件夹。
dest-foreign-deleted = 此文件夹存放的是一个已不存在的配置的备份。
dest-adopt = 将这些备份用于此配置
dest-adopt-hint = 现有备份将归此配置所有：下次备份会在其基础上继续，也可以在这里还原和合并它们。
dest-adopt-failed = 无法接管这些备份：{ $detail }
dest-foreign-hint = 目标位置存放的是另一个配置的备份：请查看“目标位置”部分。
run-foreign-destination = 目标位置存放的是另一个配置的备份。请打开配置设置：如果那个配置已被删除，可用“将这些备份用于此配置”接管它们。
browse = 浏览…
container = 存储格式
container-zip-per-file = 每个文件一个 zip
container-folder = 普通副本
container-zip-per-file-hint = 每个文件单独压缩（report.docx 变为 report.docx.zip），保留原文件夹结构；可用文件资源管理器或 7-Zip 打开。
container-folder-hint = 文件按原样复制，名称不变，不压缩。
large-files = 大文件只保存更改部分，起始大小
large-files-unit = MB
large-files-hint = 不小于此大小的文件会分块保存：文件更改后，备份只保存发生变化的块。完整备份还会保存完整文件，因此仍可用 7-Zip 打开。较小的文件按上面的格式保存。
large-files-change-hint = 可以随时更改：之后的备份将按新的阈值进行，已有的备份保持不变。
large-files-encrypted = 每个文件都分块保存，因此更改后的文件只保存变化的部分；完整备份还会保存完整副本。
schedule = 自动备份
schedule-manual = 仅在我手动启动时
schedule-daily = 每天
schedule-weekly = 每周
at = 时间
schedule-hint = 如果届时计算机处于关机状态，将在下次开机后立即运行。
merge = 合并旧备份
merge-hint = 旧备份不会被删除，而是合并到下一个保留的备份中，因此每个保留的日期都保持完整，同时释放空间。第一个完整备份永远不会被自动合并。
merge-automatic-hint = 合并是自动进行的：Archeion 运行期间会按照下面的规则合并旧备份，但绝不会在备份或其他操作正在使用它们时合并。
retention-hint = 只要有任意一条规则选中，备份就会保留。“全部，保留天数”中设定天数内的备份永远不会被合并。
keep-within = 全部，保留天数
keep-last = 最近的
keep-daily = 每天一个，保留天数
keep-weekly = 每周一个，保留周数
keep-monthly = 每月一个，保留月数
keep-yearly = 每年一个，保留年数
keep-everything = 所有规则均为 0：保留每个备份，不进行任何合并。
back-up-now = 立即备份
merge-now = 立即合并
merge-now-hint = 立即应用合并规则；第一个完整备份保持不变。
busy-hint = 已有备份或合并正在进行。
fix-profile-hint = 请先完善配置。
delete-profile = 删除配置
delete-profile-hint = 先请求确认，然后删除此配置及其计划任务；备份仍保留在目标位置。
delete-profile-title = 删除配置“{ $name }”？
delete-profile-body = 将删除此配置及其计划任务。在关闭状态栏中的消息之前，可以用“撤消”恢复它们。
delete-profile-password = 当无法再撤消时（关闭该消息、删除另一个配置或退出 Archeion 时），已保存的密码将从 Windows 凭据管理器中删除。
delete-profile-backups = { $path } 中的备份不会被删除：使用该文件夹的新配置可以通过“将这些备份用于此配置”接管它们。

day-monday = 星期一
day-tuesday = 星期二
day-wednesday = 星期三
day-thursday = 星期四
day-friday = 星期五
day-saturday = 星期六
day-sunday = 星期日

error-empty-name = 请为配置命名。
error-no-sources = 请至少添加一个要备份的文件夹。
error-no-destination = 请选择备份的保存位置。
error-destination-in-source = 目标位置位于要备份的文件夹内；请选择其他位置。

# Backups tab
refresh = 刷新
reading-backups = 正在读取备份…
no-backups-yet = 此配置还没有备份。
kind-complete = 完整
kind-incremental = 增量

# Status bar
ready = 就绪
cancel = 取消
undo = 撤消
phase-starting = 正在启动…
phase-opening = 正在打开目标位置…
phase-scanning = 正在查找更改…
phase-copying = 正在复制…
phase-merging = 正在合并旧备份…
phase-restoring = 正在还原…
phase-verifying = 正在检查…
progress-of = { $done } / { $total }
skipped-count = { $count ->
   *[other] 已跳过 { $count } 个文件（正在使用）
}
profile-removed = 已删除配置“{ $name }”。
run-finished = “{ $name }”已备份：新增 { $new }，更改 { $changed }，删除 { $deleted }，复制 { $added }。
merge-finished = “{ $name }”已合并：备份从 { $before } 个减至 { $after } 个，释放 { $freed }。
restore-finished = 已还原 { $count } 个文件。
verify-finished = 已检查 { $count } 个文件，发现 { $problems } 个问题。
run-destination-missing = 找不到目标位置：{ $location }。请连接驱动器后重试。
run-busy = 此配置的另一个备份或合并正在进行；请在其结束后重试。
run-failed = 操作失败：{ $detail }
run-no-space = 目标位置可用空间不足：需要 { $needed }，可用 { $available }。请释放空间，或合并、删除旧备份，然后重新运行：已复制的文件会保留，不会重新复制。
restore-no-space = 还原位置的可用空间不足：需要 { $needed }，可用 { $available }。未写入任何内容：请释放空间，或减少要还原的文件，或还原到其他驱动器。
backup-resumed = { $count ->
   *[other] 已沿用中断备份中的 { $count } 个文件
}
backup-as-changes = { $count ->
   *[other] { $count } 个文件只保存了更改部分
}
run-cancelled = 已取消。备份未作任何更改；已停止的备份下次会从停止处继续。

# Errors
settings-unreadable = 无法读取设置文件，因此更改将不会被保存：{ $detail }
save-failed = 无法保存设置：{ $detail }
schedule-failed = 无法安排自动运行：{ $detail }
start-failed = 无法启动操作：{ $detail }
unexpected-stop = 操作意外停止。

# Restore browser
dates = 备份日期
archive-tooltip = { $files } 个文件，此备份保存了 { $stored } 个（{ $size }）
search-hint = 在所有备份中按名称搜索文件…
no-results = 没有该名称的文件。
results = { $count ->
   *[other] 找到 { $count } 个文件
}
close = 关闭
versions-hint = 每个版本代表文件的不同内容，最新的在前。
version-modified = 修改于 { $date }
version-detail = 由 { $backup } 的备份保存 · { $size }
restore-to = 还原到：
target-beside = 原文件旁边，附加日期
target-beside-hint = report.docx 将还原为“report (2026-11-02 21.30).docx”：当前文件不受影响。
target-original = 原始位置
target-folder = 其他文件夹
replace-existing = 替换已存在的文件
replace-warning = 当前文件将被备份中的副本替换。
tick-files-hint = 请至少勾选一个文件或文件夹。
choose-folder-hint = 请选择要还原到的文件夹。
restore-selected = 还原
clear-selection = 清除选择
tick-folder = 还原此文件夹及其全部内容
tick-file = 还原此文件
backups-restore-files = 还原文件
backups-restore-files-hint = 曾经保存过的所有文件：勾选要找回的内容，选择版本或日期以及还原位置。
backups-manage = 管理备份
backups-manage-hint = 备份日期列表：将多个备份合并为一个，或删除其中一个。
restore-tree-count = 备份中共 { $files } 个文件 · 已从源中删除：{ $deleted }
restore-tree-hint = 勾选文件或文件夹即可还原；搜索可按名称查找文件。标有 🗑 已删除 的文件已不在源中，但仍可还原。
deleted-label = 已删除
deleted-since = 不在最新的备份中：已从源中删除。
tree-deleted-file = 不在最新的备份中：已从源中删除。勾选它以还原其某个版本。
tree-deleted-folder = 不在最新的备份中：已从源中删除。勾选它以将其还原为某个日期时的状态。
remove-from-selection = 从所选内容中移除
selection-empty = 尚无要还原的内容。在树中勾选文件或文件夹：每一项都会显示在这里，附带其版本或日期以及还原位置。
folder-as-of = 还原为此日期的状态：
folder-as-of-hint = 文件夹将整体还原，包含该日期时的全部内容。
folder-size = 该日期时为 { $size }
dest-default = 默认：{ $dest }
dest-follow-default = 使用默认还原位置
dest-overwrite = 覆盖原文件
dest-folder = 其他文件夹…
dest-folder-missing = 其他文件夹（尚未选择）
dest-change-hint = 此项的还原位置。✎ 表示与默认位置不同。
restore-to-hint = 适用于所有勾选的内容，除非在下方的版本或文件夹中单独更改。
restore-conflict = 另一个版本也会覆盖同一个原文件：“覆盖原文件”只能用于一个版本。
restore-conflict-hint = 两个版本将覆盖同一个原文件：请更改其中一个的还原位置。
restore-wait-versions = 正在读取已勾选文件的版本…
restore-summary = { $files } 个文件，{ $versions } 个版本，{ $folders } 个文件夹 — { $size }

# Manual merge
merge-choose = 选择要合并的备份…
merge-choose-hint = 勾选第一个和最后一个备份：两者之间的所有备份将合并为一个。
merge-tick = 勾选为合并范围的端点
merge-tick-hint = 勾选要合并的最早和最新的备份；两者之间的备份都会包括在内。
merge-summary = { $count } 个备份将合并为一个，日期为 { $date }。
merge-complete-warning = 该范围包括第一个完整备份：它将被一个新的完整备份取代，日期为所选的最新备份，之前的日期将无法再还原。
merge-complete-ack = 我已了解
merge-two-hint = 请至少勾选两个备份。
merge-ack-hint = 请确认你已了解此警告。
merge-selected = 合并

# Activity tab
tab-activity = 活动
show-details = 详细信息
cancel-hint = 立即停止：备份将保持本次运行前的状态。下次备份会重用已复制的文件。
activity-backup-running = 正在备份“{ $name }”
activity-merge-running = 正在合并“{ $name }”的备份
activity-restore-running = 正在从“{ $name }”还原
activity-preparing = 正在准备…
activity-file = 文件
activity-folder = 文件夹
activity-files = 文件
activity-speed = 速度
activity-elapsed = 已用时间
activity-remaining = 剩余时间
count-of = { $done } / { $total }
files-found = 目前已找到 { $count } 个
about = 约 { $time }
activity-took = 用时 { $time }。
activity-backup-summary = 完成：新增 { $new }，更改 { $changed }，删除 { $deleted }，未更改 { $unchanged }；复制 { $copied }。
activity-merge-summary = 完成：备份从 { $before } 个减至 { $after } 个，释放 { $freed }。
activity-restore-summary = 完成：已还原 { $restored } 个文件，{ $kept } 个已存在并保留。
skipped-hint = 无法读取这些文件，通常是因为其他程序正在打开它们。它们的上一个版本（如有）仍在备份中。
skipped-more = …还有 { $count } 个（见日志）。

# Exclusions and format lock
no-exclusions = 未排除任何内容：将备份文件夹中的所有文件。
remove-exclusion = 重新备份此项
exclusion-folder = 文件夹及其全部内容
exclusion-file = 文件
exclusion-type = 此类型的所有文件
exclusion-pattern = 模式
exclude-folder = 排除文件夹…
exclude-file = 排除文件…
exclude-type-hint = .tmp
exclude-type = 排除类型
common-types = 常用：
advanced = 高级
exclude-pattern = 按模式排除
exclude-pattern-add = 添加
container-locked = 目标位置已有此格式的备份：格式无法更改。要使用另一种格式，请选择一个空的目标位置。
container-unreachable = 无法访问目标位置，因此现在无法更改格式。
container-checking = 正在检查目标位置…
activity-idle-title = 当前没有运行的操作
activity-idle-body = 备份、合并和还原的进度将显示在这里。

# Backup kinds and deletion
menu-run-incremental = 立即增量备份
menu-run-complete = 立即完整备份
run-now = 立即运行：
run-incremental = 增量
run-incremental-hint = 只复制新增和更改的文件；速度快，是常用选择。
run-complete = 完整
run-complete-hint = 复制所有文件并开始新的链：之后的备份不再依赖之前的备份。耗时更长，占用空间更多。
run-merge = 合并
run-first = 运行第一次备份
run-incremental-button = 运行增量备份
run-complete-button = 运行完整备份
full-backup = 完整备份：
full-never = 仅第一次
full-every-backups = 每隔
full-every-days = 当上次完整备份早于
full-backups-unit = 次备份
full-days-unit = 天
full-backup-hint = 适用于自动备份；完整备份会开始新的链，使还原不再依赖更早的备份。
run-finished-complete = “{ $name }”已完整备份：新增 { $new }，更改 { $changed }，删除 { $deleted }，复制 { $added }。
activity-complete-summary = 完整备份完成：新增 { $new }，更改 { $changed }，删除 { $deleted }，未更改 { $unchanged }；复制 { $copied }。
activity-delete-running = 正在删除“{ $name }”的一个备份
activity-delete-summary = 完成：备份从 { $before } 个减至 { $after } 个，释放 { $freed }。
delete-finished = 已删除“{ $name }”的一个备份。
delete-backup = 删除此备份…
delete-backup-hint = 删除所选日期；之后的备份仍保持完整。
delete-backup-confirm = 删除备份
delete-initial-hint = 只有当第一个完整备份是唯一的备份时才能删除它：其他备份都依赖它。
delete-complete-hint = 完整备份是一条链的起点，之后的备份都依赖它：无法删除。请改为将它与之后的备份合并。
delete-only-warning = { $date } 的备份是唯一的备份：目标位置将变为空，下次备份将是完整备份。此操作无法撤消。
delete-newest-warning = 将删除 { $date } 的备份，以及只保存在其中的文件版本。此操作无法撤消。
delete-middle-warning = 将删除 { $date } 的备份。之后的备份所需的文件会移入下一个备份；只保存在此备份中的版本将丢失。此操作无法撤消。

# Encryption
encrypt = 加密备份（AES-256，包括文件名）
encrypt-hint = 每个文件都是带有隐藏名称的 AES-256 zip；没有密码，任何人都无法读取内容，也看不到保存了哪些文件。密码保存在 Windows 凭据管理器中。
encrypt-needs-zip = 加密需要“每个文件一个 zip”格式。
password = 密码
password-again = 再次输入密码
password-set = 设置密码
password-reenter = 输入这些备份的密码
password-saved = 密码已保存在 Windows 凭据管理器中。
password-missing = 此计算机上没有保存密码：请输入密码以运行和浏览备份。
password-loss-warning = 没有此密码或首次备份后显示的恢复密钥，备份将无法打开，任何人都无法恢复。请将密码保存在安全的地方。
password-save = 保存密码
password-checking = 正在检查密码…
password-empty-hint = 请输入密码。
password-short-hint = 请至少使用 { $count } 个字符。
password-mismatch-hint = 两次输入的密码不同。
password-rule-length = 至少 { $count } 个字符（当前 { $typed } 个）
password-rule-match = 两次输入的密码一致
password-rule-variety = 在小写字母、大写字母、数字和符号中至少包含 { $kinds } 类（长度达到 { $passphrase } 个字符时无需满足）
password-rule-common = 不是常见密码，例如“Password2024”或“qwerty”
password-rule-runs = 不含 { $count } 个重复或连续的字符，例如“aaaa”“1234”或“abcd”
password-rule-name = 不包含配置名称
password-rule-strength = 足够难猜：强度至少为“{ $level }”
password-rule-tip = 可以使用任何字符，包括空格：由四五个词组成的句子既好记又难猜；长度达到 { $passphrase } 个字符时，无需数字、大写字母或符号。
password-strength = 强度：{ $level }
password-strength-weak = 弱
password-strength-fair = 中
password-strength-strong = 强
password-strength-very-strong = 非常强
password-variety-hint = 混合使用小写字母、大写字母、数字和符号，或写一个至少 { $passphrase } 个字符的句子。
password-common-hint = 这是最常用的密码之一：请换一个。
password-runs-hint = 避免使用“aaaa”或“1234”这样的序列。
password-name-hint = 不要在密码中包含配置名称。
password-weak-hint = 太容易猜到：请加长密码，例如再加几个词。
password-first-hint = 请在上方输入加密备份的密码（新备份需输入两次，并勾选“我已了解”）：该按钮会保存密码并开始备份。
password-wrong = 此密码无法打开目标位置中的备份。
password-unreachable = 无法检查密码
run-password-needed = 备份已加密，但此计算机上没有保存密码：请在配置设置中输入密码。
run-wrong-password = 已保存的密码无法打开备份：请在配置设置中重新输入。

# Operations log
log-runs = 操作
log-only-profile = 仅此配置
log-running = 进行中
log-empty = 还没有操作。
log-pick = 请在左侧选择一个操作。
log-title = 日志
log-no-lines = 尚无日志记录。
log-scheduled = 自动
log-manual = 手动启动
log-started = 开始于 { $time }
log-backup = 备份 { $name }
log-unfinished = 未完成：已中断，或仍在运行。
log-skipped = 已跳过 { $path }：{ $reason }
log-op-backup = 备份
log-op-incremental = 增量备份
log-op-complete = 完整备份
log-op-merge = 合并
log-op-delete = 删除备份
log-op-restore = 还原
log-open-full = 打开完整日志
log-open-full-hint = 将此操作的完整日志写入文本文件并在你的文本编辑器中打开：每个步骤，然后是它处理的每个文件及其处理方式，包括跳过的文件。
log-copy = 复制日志
log-copy-hint = 以文本形式复制此操作的完整日志，以便粘贴到电子邮件或问题报告中。
log-copied = 已复制日志。
log-open-program = 程序日志
log-open-program-hint = 打开 archeion.log，即程序的技术日志：可用于排查问题或报告问题。
log-open-folder = 日志文件夹
log-open-folder-hint = 在文件资源管理器中打开操作日志所在的文件夹。
log-program-missing = 程序日志尚不存在。
log-unreadable = 无法读取日志文件。
log-report-failed = 无法显示完整日志：{ $detail }
log-level-info = 信息
log-level-done = 完成
log-level-warning = 警告
log-level-error = 错误
log-preparing = 正在准备完整日志…
log-files-title = 文件
log-files-missing = 此操作未记录文件列表：它由旧版本的 Archeion 执行。
log-files-none = 未处理任何文件。
log-file-new-chunks = 保存为更改：写入了 { $size } 的新数据块
log-file-resumed = 已由中断的备份复制，予以保留
log-count-new = 新增：{ $count }
log-count-changed = 已更改：{ $count }
log-count-as-changes = 保存为更改：{ $count }
log-count-copied-again = 再次复制：{ $count }
log-count-unchanged = 未更改，未列出：{ $count }
log-count-deleted = 已删除：{ $count }
log-count-moved = 已移动：{ $count }
log-count-dropped = 已移除的版本：{ $count }
log-count-restored = 已还原：{ $count }
log-count-kept = 已保留：{ $count }
log-count-checked = 读回完好：{ $count }
log-count-checked-unlisted = 读回完好，未列出：{ $count }
log-count-damaged = 问题：{ $count }
log-count-skipped = 已跳过：{ $count }

# Password change and recovery key
password-change = 更改密码
password-new = 新密码
password-change-button = 更改密码
recovery-later = 首次备份后即可获得恢复密钥。
recovery-show = 显示恢复密钥…
recovery-title = 恢复密钥
recovery-hint = 使用此密钥，即使没有 Archeion 和密码，7-Zip 也能打开每个加密文件和清单。每个备份的清单都列出了每个文件的存放位置。
recovery-warning = 任何拥有此密钥的人都能读取所有备份。请将其打印或抄写下来并离线保存；更改密码时它不会改变。
recovery-copy = 复制
recovery-copy-hint = 按 7-Zip 要求的格式复制密钥，不含空格。
recovery-hide = 隐藏
password-current = 当前密码
password-current-hint = 请输入当前密码。
recovery-ask = 要显示恢复密钥，请输入这些备份的当前密码。
recovery-show-button = 显示

# Integrity check
verify-section = 检查
verify-hint = 完整检查会重新读取每个备份中保存的每个文件，并与其指纹进行比较；还原测试会随机重新读取最新备份中的部分文件。到期的检查会在计划备份之后运行。
verify-full-every = 完整检查间隔
verify-sample-every = 还原测试间隔
verify-sample-files = 文件数：
verify-zero-hint = 0 天 = 从不自动进行。
verify-full-now = 立即检查备份
verify-full-hint = 重新读取并检查每个保存的文件；目标位置较大时，所需时间与读取全部备份相同。
verify-sample-now = 立即进行还原测试
verify-sample-hint = 像还原一样随机重新读取最新备份中的部分文件，但不会将其写入任何位置。
verify-no-backups = 还没有可检查的备份。
verify-problems = 发现问题
verify-problems-hint = 所列备份可能无法正确还原。请立即进行一次完整备份，并检查目标驱动器。
activity-verify-running = 正在检查“{ $name }”的备份
log-op-verify = 完整性检查
log-op-restore-test = 还原测试

# Automatic backup: schedules, events, notifications
schedule-days = 在选定的日子
schedule-hours = 每隔几小时
schedule-monthly = 每月一次
schedule-all-days = 每天
schedule-add-time = + 添加时间
schedule-remove-time = 删除此时间
schedule-every = 每隔
hours-unit = 小时
schedule-from = 从
schedule-until = 至
schedule-hours-hint = 最后一次运行在结束时间之前开始；开始和结束时间相同表示全天。
schedule-month-day = 日期
schedule-month-hint = 仅限 1 至 28 日，以确保每个月都有这一天。
events-title = 以下情况也进行备份：
event-logon = 登录 Windows 几分钟后
event-idle = 计算机空闲 10 分钟时
event-drive = 连接驱动器时
event-drive-hint = 适用于以外部驱动器作为目标位置的情况：插入该驱动器时开始备份，插入其他驱动器时不会有任何动作。
event-min-hours = 最多每隔
event-hint = 如果上次备份更新，或目标位置未连接，则跳过由这些事件触发的备份。
alerts = 通知
alert-on-failure = 自动备份失败时通知我
alert-stale = 超过以下时间没有备份时提醒我
alert-hint = Windows 通知；0 天 = 不提醒。每天以及登录后都会进行检查。
alert-manual-hint = 仅适用于自动备份：请在“自动备份”中选择计划。
alert-failed-title = Archeion：“{ $name }”备份失败
alert-stale-title = Archeion：“{ $name }”尚未备份
alert-stale-body = 已有 { $days } 天没有备份（上次于 { $when }）。请检查目标位置是否已连接，或在 Archeion 中开始备份。
closing-after-run = 有操作正在进行：Archeion 将在其结束后自动关闭。
closing-cancel = 不要关闭

# Notification area and Preferences window
menu-preferences = 首选项…
preferences-title = 首选项
preferences-window = 窗口
language = 语言
language-system = 系统语言
language-tray-restart = 通知区域图标的菜单将在下次启动 Archeion 时更改语言。
quit-hint = 关闭 Archeion；计划的备份仍会通过 Windows 任务计划程序运行。
tray-open = 打开 Archeion
tray-quit = 退出
tray-running = 操作进行中
option-tray = 关闭窗口后继续在通知区域中运行
option-tray-hint = 关闭窗口会将其隐藏到时钟旁：Archeion 会继续合并旧备份并完成正在进行的操作。单击其图标可再次打开窗口，或选择“退出”。
option-tray-unavailable = Windows 未提供通知区域图标。
option-autostart = 随 Windows 启动 Archeion
option-autostart-hint = 登录时，Archeion 会在通知区域中隐藏启动。也可以在任务管理器的“启动应用”中关闭此项。
autostart-failed = 无法更改随 Windows 启动设置：{ $detail }

# Help menu: user guide and About
menu-documentation = 文档
menu-about = 关于 Archeion
about-version = 版本 { $version }
about-name = 名称源自古希腊语 ἀρχεῖον，意为“档案”。
about-author = 作者：MR
about-license = 许可证：GPL-3.0
about-repository = GitHub 上的源代码
about-logs-folder = 日志文件夹
about-open-folder = 打开文件夹
about-folder-missing = 该文件夹尚不存在。
doc-title = Archeion 文档
doc-intro = Archeion 会将你的文件夹复制到另一个驱动器、网络共享或任何其他文件夹，并能将任何文件恢复为过去某个日期时的样子。打开一个部分即可阅读。
doc-what-title = Archeion 的功能
doc-what-backups = 配置的第一次备份是完整备份：复制所有文件。之后的备份是增量备份：只复制新增和更改的文件，因此速度快、占用小，但每个日期仍可完整还原。在配置设置的“完整备份”中，还可以设置每隔若干次备份，或在上次完整备份早于若干天时，进行一次新的完整备份；完整备份会开始新的链，不再依赖更早的备份。
doc-what-storage = 每个文件单独存储，位于与原文件相同的文件夹中：可以是该文件的 zip（“每个文件一个 zip”：report.docx 变为 report.docx.zip）、普通副本，或 AES-256 加密的 zip。无需 Archeion 也能读取备份：文件资源管理器和 7-Zip 可以打开 zip，7-Zip 可以用恢复密钥打开加密的 zip。
doc-what-safe = 中断的备份（计算机关机、驱动器被移除、点击“取消”）不会改变现有备份；下次备份会从停止处继续，不会重新复制已复制的文件。
doc-profiles-title = 配置
doc-profiles-body = 配置规定了备份什么、备份到哪里以及何时备份。通过“文件 > 新建配置”（Ctrl+N）创建配置，并填写其“设置”选项卡：名称、要备份的文件夹和目标位置（文件夹、外部驱动器或网络共享）。每个目标文件夹只用于一个配置。
doc-profiles-exclusions = 排除项可以略过你不需要的内容：某个文件夹及其全部内容、某个文件，或某种类型（例如 .tmp）的所有文件。在“按模式排除”中可以添加模式，模式匹配完整路径，不区分大小写：* 表示名称中的任意文本，** 表示任意层文件夹，? 表示一个字符。例如 **/node_modules 或 **/cache/**。
doc-profiles-format = 存储格式（每个文件一个 zip 或普通副本）和加密必须在第一次备份之前选定：目标位置中一旦有了备份，就无法再更改。要更换，请选择一个空的目标位置。
doc-profiles-delete = “删除配置”会先请求确认，然后删除此配置及其计划任务；备份仍保留在目标位置。删除后可立即用状态栏中的“撤消”恢复配置。之后使用该文件夹的配置可以通过“将这些备份用于此配置”接管这些备份。
doc-auto-title = 自动备份
doc-auto-schedule = 在“自动备份”中选择备份自动运行的时间：在选定的日子于一个或多个时间运行、在两个时间之间每隔几小时运行，或每月在 1 至 28 日中的某一天运行一次。Archeion 会在 Windows 任务计划程序中注册计划，因此即使 Archeion 未打开，备份也会运行。
doc-auto-missed = 如果在计划时间计算机处于关机状态，备份将在下次开机后立即运行。
doc-auto-alerts = 在“通知”中，Archeion 可以在自动备份失败时显示 Windows 通知，并在某个配置已有若干天没有备份时发出提醒（每天以及登录后检查）。
doc-auto-now = 你随时可以手动开始备份：使用配置设置中的“立即备份”，或者“备份 > 立即增量备份”（Ctrl+B）和“立即完整备份”。
doc-merge-title = 合并旧备份
doc-merge-body = 旧备份不会被删除，而是合并到下一个保留的备份中：每个保留的日期都保持完整，不再需要的版本所占的空间会被释放。合并是自动进行的：在 Archeion 运行期间进行，但绝不会在备份或其他操作正在使用目标位置时进行。
doc-merge-rules = “合并旧备份”中的规则决定保留哪些备份，只要有任意一条规则选中，备份就会保留：最近若干天内的全部备份、最近的若干个、每天一个、每周一个、每月一个和每年一个，各自按你设定的数量保留。所有规则均为 0 时，保留所有备份，不进行任何合并。
doc-merge-first = 第一个完整备份永远不会被自动合并。
doc-merge-manual = 在“备份 > 管理备份”中，“选择要合并的备份…”可以手动合并：勾选最早和最新的备份，两者之间的所有备份将合并为一个。如果范围包括第一个完整备份，Archeion 会先发出警告，因为之前的日期将无法再还原。
doc-merge-delete = “备份 > 管理备份”中的“删除此备份…”会删除一个日期；之后的备份仍保持完整。之后的备份所依赖的完整备份无法删除（请改为合并）。删除操作无法撤消。
doc-encrypt-title = 加密
doc-encrypt-body = 启用“加密备份”后，每个文件都会变成带有隐藏名称的 AES-256 zip：没有密码，任何人都无法读取内容，也看不到保存了哪些文件。加密需要“每个文件一个 zip”格式，并且必须在第一次备份之前启用；之后无法更改。
doc-encrypt-password = 密码保存在 Windows 凭据管理器中，因此自动备份无需输入密码。以后可以在配置设置中输入当前密码来更改它。在另一台计算机上，请在配置设置中输入一次密码。
doc-encrypt-recovery = 第一次备份后，“显示恢复密钥…”会显示一个密钥；使用它，即使没有 Archeion 和密码，7-Zip 也能打开每个加密文件。请将其打印或抄写下来并离线保存：既没有密码也没有密钥，任何人都无法打开备份。更改密码时密钥不会改变。Archeion 在显示密钥前会要求输入当前密码。
doc-large-title = 大文件
doc-large-body = 变化不大的大文件（邮箱、虚拟机磁盘、数据库）不会每次都完整复制：它被分成约 1 MB 的块保存，每次备份只保存新的块。相同的块只保存一次。这适用于不小于配置“存储格式”部分所选大小的文件（默认 500 MB，可以关闭），以及加密目标中的所有文件。
doc-large-complete = 完整备份仍以 zip、普通副本或加密 zip 的形式保存每个完整文件，因此只用 7-Zip 就能还原；只有其后的增量备份包含分块。还原时会重新拼合各块并逐块检查；完整性检查会读取所有块。
doc-large-upgrade = 旧版本创建的备份保持原样，仍可读取。分块从下一次完整备份开始：在这样的目标上，下一次自动备份将是完整备份。
doc-restore-title = 还原文件
doc-restore-body = 打开配置的“备份”选项卡（或“备份 > 还原文件…”）：“还原文件”以一棵树显示备份曾保存过的所有文件和文件夹；已不在源中的文件标有 🗑 已删除，但仍可还原。在树中或搜索结果中勾选文件或文件夹（搜索框可按名称查找文件）。每个勾选的项目都会显示在右侧；单击 ✕ 可将其移除。
doc-restore-versions = 勾选的文件会列出其版本（最新的在前），每个版本内容不同：最新版本已勾选，你还可以勾选更多版本。勾选的文件夹会整体还原为“还原为此日期的状态”中所选日期时的状态（默认为最新日期）。
doc-restore-where = 在右侧面板顶部选择所有内容的还原位置：原文件旁边，名称中附加日期（report.docx 将还原为“report (2026-11-02 21.30).docx”，当前文件不受影响）、原始位置，或其他文件夹；只有勾选“替换已存在的文件”时，现有文件才会被替换。每个版本或文件夹都可以通过自己的菜单还原到别处，并以 ✎ 标记。同一文件的多个版本还原到同一文件夹时，每个版本的名称中都会附加各自的日期；一个文件只能有一个版本覆盖原文件。单击“还原”即可在一次操作中全部还原。
doc-verify-title = 完整性检查和还原测试
doc-verify-body = 完整检查会重新读取每个备份中保存的每个文件，并与其指纹进行比较；还原测试会像还原一样随机重新读取最新备份中的部分文件，但不会将其写入任何位置。可以在配置设置的“检查”部分（或“备份 > 立即检查备份”）中启动它们，也可以让它们在计划备份之后每隔几天运行一次。如果发现问题，请进行一次完整备份并检查目标驱动器。
doc-activity-title = “活动”选项卡
doc-activity-body = “活动”选项卡显示正在运行的备份、合并、还原或检查的进度，并带有“取消”按钮，还显示过去每个操作（自动或手动启动）的日志：操作做了什么，以及哪些文件因其他程序正在打开而被跳过。在操作步骤上方，“打开完整日志”会在你的编辑器中以文本文件形式显示其完整日志：各个步骤，然后是它处理的每个文件，包括时间、处理结果（新增、已更改、保存为更改、再次复制、已删除、已移动、已移除旧版本、已还原、已保留、已跳过、问题）、大小和完整路径；增量备份中未更改的文件和完整检查中完好的文件只计数。“复制日志”复制同样的文本；“程序日志”会打开 Archeion 的技术日志，用于排查更深层的问题。
doc-window-title = 关闭窗口
doc-window-body = 关闭窗口不会停止 Archeion：它会移到时钟旁的通知区域并继续工作，因此旧备份仍会被合并；可以从那里的图标再次打开窗口。Archeion 还会随 Windows 启动，并隐藏在那里。“文件 > 退出”或图标菜单中的“退出”才会真正关闭它。这两项都可以在“文件 > 首选项”中关闭，你也可以在那里选择语言。无论如何，计划的备份都会运行，即使 Archeion 没有运行。
doc-cli-title = 命令行
doc-cli-body = 不带参数运行 archeion.exe 会打开窗口。下面的命令会在不打开窗口的情况下运行一个操作，就像计划任务那样。配置 ID 是 settings.toml 中该配置的 id。
doc-cli-run = 备份该配置（--kind complete 表示完整备份）。
doc-cli-merge = 按配置的规则合并旧备份。
doc-cli-verify = 检查备份（--sample N：对 N 个文件进行还原测试）。
doc-cli-delete = 删除一个备份。
doc-cli-restore = 还原由窗口写入的请求文件中列出的文件。
doc-cli-watch = 提醒哪些配置最近没有备份；任务计划程序每天运行它。
doc-cli-extract = 仅凭备份文件夹进行还原，无需设置或配置：例如在另一台计算机上使用便携版 Archeion。文件以完整路径放在目标文件夹下。
doc-cli-extract-options = extract 的选项：--archive <名称> 选择备份（省略时为最新的）；--path <文件或文件夹> 只还原以原始路径指定的内容；--overwrite 替换已有文件。对于加密备份，请通过 --stdin（标准输入的第一行）、环境变量 ARCHEION_PASSWORD 或 ARCHEION_RECOVERY_KEY，或者 --password 或 --recovery-key 提供密码或恢复密钥。进度也会写入目标文件夹中的 archeion-extract.log。在命令提示符中，请以 start /wait archeion extract … 运行，使命令提示符等待其完成。
doc-cli-exit = 退出代码：0 成功，1 失败，2 找不到目标位置，3 忙（另一个操作正在使用目标位置），4 密码缺失或错误，5 可用空间不足。
doc-data-title = 数据和日志的位置
doc-data-body = 设置位于 %APPDATA%\Archeion\config 文件夹（settings.toml）中，每个操作的日志位于其 logs 子文件夹中。便携版（archeion.exe 旁有 portable.txt 文件）则将所有内容保存在可执行文件旁的 data 文件夹中。“帮助 > 关于 Archeion”可以打开这些文件夹。
doc-data-backups = 备份本身只存放在目标位置：每个备份一个文件夹，以其日期和时间命名，其中包含保存的文件以及当时所有文件的列表。

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
   *[other] 找到 { $count } 个文件
}
step-folders = { $count ->
   *[other] 已读取 { $count } 个文件夹
}
step-new = 新增 { $count }
step-changed = 更改 { $count }
step-unchanged = 未更改 { $count }
step-copied-again = 未更改但重新复制 { $count }
step-moved = { $count ->
   *[other] 已移动 { $count } 个文件
}
step-restored = 已还原 { $count }
step-kept = 已存在并保留 { $count }
step-checked = 已检查 { $count }
step-damaged = { $count ->
   *[other] { $count } 个问题
}
step-skipped = 已跳过 { $count }
merge-group = { $count } 个备份（{ $first } 至 { $last }）合并为一个
merge-group-of = 第 { $index } 组，共 { $of } 组：{ $count } 个备份（{ $first } 至 { $last }）合并为一个
recent-title = 最近的文件
recent-new = 新增
recent-changed = 已更改
recent-copied-again = 重新复制
recent-moved = 已移动
recent-restored = 已还原
recent-kept = 已保留
recent-checked = 正常
recent-damaged = 问题
recent-skipped = 已跳过
recent-deleted = 已删除
recent-dropped = 已移除旧版本
