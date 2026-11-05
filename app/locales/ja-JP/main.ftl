# Menus
menu-file = ファイル
menu-backup = バックアップ
menu-help = ヘルプ
new-profile = 新しいプロファイル
quit = 終了
restore = ファイルを復元…
data-folder = データ フォルダー

# Profile list and empty state
profiles = プロファイル
unnamed = （名前なし）
new-profile-name = 新しいバックアップ
empty-title = バックアップはまだありません
empty-body = プロファイルを作成して、保存するフォルダー、保存先、実行するタイミングを選びます。
tab-settings = 設定
tab-backups = バックアップ

# Profile form
profile-name = 名前
folders = バックアップするフォルダー
no-folders = フォルダーがまだありません。少なくとも 1 つ追加してください。
add-folder = フォルダーを追加…
remove-folder = このフォルダーをバックアップから外す
exclusions = 除外
exclusions-hint = パターンは大文字と小文字を区別せず、フルパスと照合されます。* は名前の中の任意の文字列、** は任意のフォルダー、? は 1 文字です。例: **/node_modules、**/cache/**、**/~$*。
destination = 保存先
dest-local = フォルダーまたは外付けドライブ
dest-network = ネットワーク共有
dest-hint = このプロファイル専用のフォルダーを使ってください。暗号化 (下記) を有効にしない限り、そのフォルダーを開ける人は誰でもバックアップを読めます。
dest-free = 保存先の空き容量: { $free }
dest-fs-exfat = このドライブは exFAT で、ジャーナルがありません。バックアップや統合の途中で停電したりドライブを外したりすると、バックアップが壊れることがあります。NTFS の方が安全です。
dest-fs-fat = このドライブは FAT32 です。4 GiB 以上のファイルはバックアップできず、ジャーナルもないため、バックアップや統合の途中で停電するとバックアップが壊れることがあります。NTFS または exFAT でフォーマットしてください。
dest-foreign-profile = このフォルダーにはプロファイル「{ $name }」のバックアップがあります。このプロファイルには別のフォルダーを選んでください。
dest-foreign-deleted = このフォルダーには、すでに存在しないプロファイルのバックアップがあります。
dest-adopt = これらのバックアップをこのプロファイルで使う
dest-adopt-hint = 既存のバックアップがこのプロファイルのものになります。次回のバックアップはその続きとなり、ここから復元や統合ができます。
dest-adopt-failed = バックアップを引き継げませんでした: { $detail }
dest-foreign-hint = 保存先には別のプロファイルのバックアップがあります。「保存先」セクションを確認してください。
run-foreign-destination = 保存先には別のプロファイルのバックアップがあります。プロファイルの設定を開いてください。そのプロファイルが削除済みなら、「これらのバックアップをこのプロファイルで使う」で引き継げます。
run-encryption-missing = 保存先のバックアップは暗号化済みとして記録されていませんが、プロファイルまたはバックアップ自体は暗号化されています。最初のバックアップの後に暗号化を有効にしたか、保存先の説明が Archeion の外で削除または変更されました。何も書き込んでいません。暗号化したバックアップには、新しい空の保存先を選んでください。
network-unconfirmed = このプロファイルはインポートされたもので、ほかのコンピューター上のフォルダー（{ $hosts }）を使います。Archeion はまだ接続していません。接続すると Windows があなたのアカウントでサインインします。知っているコンピューターであることを確認してから承認してください。
network-confirm = これらのネットワークフォルダーを使う
run-network-unconfirmed = このインポートしたプロファイルは、まだ承認されていないほかのコンピューター上のフォルダーを使います。設定を開いて「これらのネットワークフォルダーを使う」を押してください。
browse = 参照…
container = 保存形式
container-zip-per-file = ファイルごとに zip
container-folder = そのままコピー
container-zip-per-file-hint = 各ファイルを個別に圧縮し (report.docx は report.docx.zip になります)、同じフォルダー構成で保存します。エクスプローラーや 7-Zip で開けます。
container-folder-hint = ファイルを圧縮せず、同じ名前のままコピーします。
large-files = 大きなファイルを変更分だけ保存する。対象サイズ:
large-files-unit = MB 以上
large-files-hint = このサイズ以上のファイルは分割して保存されます。変更後のバックアップには、変更された部分だけが保存されます。完全バックアップにはファイル全体も保存されるため、7-Zip で開くことができます。これより小さいファイルは上記の形式で保存されます。
large-files-change-hint = いつでも変更できます。以降のバックアップは新しいしきい値に従い、作成済みのバックアップはそのまま残ります。
large-files-encrypted = すべてのファイルが分割して保存されるため、変更されたファイルは変更された部分だけが保存されます。完全バックアップにはファイル全体のコピーも保存されます。
schedule = 自動バックアップ
schedule-manual = 手動で開始したときのみ
at = 時刻
time-hour = 時
time-minute = 分
schedule-hint = その時刻にコンピューターの電源が切れていた場合は、次に起動したときすぐに実行します。
schedule-paused = このプロファイルはインポートされたもので、スケジュールはまだ動作しません。何をどこにバックアップするか確認してから、有効にしてください。
schedule-resume = スケジュールを有効にする
merge = 古いバックアップの統合
merge-hint = 古いバックアップは削除されず、次に残すバックアップに統合されます。そのため残した日付はすべて完全なまま、空き容量が増えます。最初の完全バックアップが自動で統合されることはありません。
merge-enabled = 古いバックアップを自動的に結合する
merge-enabled-hint = オフにすると、バックアップが自動で結合されることはありません。手動で結合または削除するまで、すべて残ります。
merge-off-hint = 自動結合はオフです。[バックアップ] > [バックアップの管理] で結合または削除するまで、各バックアップは残ります。
merge-off-run-hint = 自動結合はオフです。[バックアップ] > [バックアップの管理] で手動で結合してください。
merge-automatic-hint = 統合は自動で行われます。Archeion の実行中、下のルールに従って古いバックアップを統合します。バックアップや他の操作が使用している間は統合しません。
merge-keep-deleted = 元の場所で削除されたファイルを統合後のバックアップに残す
merge-keep-deleted-hint = 統合するバックアップの間に削除されたファイルは、最後の内容のまま統合後のバックアップに残るため、引き続き復元できます。チェックを外すと破棄され、その分の領域が空きます。
retention-hint = いずれかのルールで選ばれたバックアップは残されます。「すべて (日数)」で指定した日数内のバックアップは統合されません。
merge-preview = { $count ->
   *[other] この規則では、次の結合で { $count } 件のバックアップ ({ $from } ～ { $to }) が残る次のバックアップに結合されます。
}
merge-preview-none = この規則では、今は結合するものはありません。
merge-waits = { $count ->
   *[other] 新しい規則では、以前の規則で残っていた { $count } 件のバックアップが結合されます。自動結合はあなたの確認を待ちます。
}
merge-confirm = 新しい規則で結合
merge-confirm-hint = 結合は元に戻せません。それらのバックアップのバージョンは失われます。
merge-restore-rules = 以前の規則に戻す
keep-within = すべて (日数)
keep-last = 最新
keep-daily = 1 日に 1 つ (日数)
keep-weekly = 1 週間に 1 つ (週数)
keep-monthly = 1 か月に 1 つ (月数)
keep-yearly = 1 年に 1 つ (年数)
keep-everything = すべてのルールが 0 です。すべてのバックアップを残し、何も統合しません。
merge-now = 今すぐ統合
merge-now-hint = 統合ルールを今すぐ適用します。最初の完全バックアップはそのまま残ります。
busy-hint = バックアップまたは統合がすでに実行中です。
fix-profile-hint = 先にプロファイルを完成させてください。
delete-profile = プロファイルを削除
delete-profile-hint = 確認したうえで、プロファイルとそのスケジュールされたタスクを削除します。バックアップも削除することを選ばない限り、バックアップは保存先に残ります。
profile-item-hint = ドラッグで順序を変更、右クリックで名前の変更や削除。
rename-profile = 名前の変更
duplicate-profile = 複製
duplicate-profile-hint = 保存先以外のすべての規則を持つコピーです。保存先はあなたが選びます (保存先フォルダーは 1 つのプロファイル専用です)。
profile-copy-name = { $name } (コピー)
profile-copy-name-n = { $name } (コピー { $n })
move-profile-up = 上へ移動
move-profile-down = 下へ移動
move-profile-first = すでに先頭です。
move-profile-last = すでに末尾です。
delete-profile-menu = プロファイルを削除…
key-delete = Delete
delete-profile-title = プロファイル「{ $name }」を削除しますか?
delete-profile-body = プロファイルとそのスケジュールされたタスクが削除されます。ステータス バーのメッセージを閉じるまでは、「元に戻す」で復元できます。
delete-profile-password = 保存されたパスワードは、元に戻せなくなった時点 (そのメッセージを閉じたとき、別のプロファイルを削除したとき、または Archeion を終了したとき) で Windows 資格情報マネージャーから削除されます。
delete-profile-backups = { $path } のバックアップは削除されません。そのフォルダーを使う新しいプロファイルで、「これらのバックアップをこのプロファイルで使う」を使って引き継げます。
delete-profile-with-backups = { $path } のバックアップもごみ箱に移動する
delete-profile-backups-gone = ごみ箱を空にするまではそこから元に戻せますが、[元に戻す] でプロファイルは戻せなくなります。移動するのは Archeion が書き込んだものだけで、フォルダー内のほかのファイルは残ります。ネットワーク共有やリムーバブル ドライブにはごみ箱がないため、そこでは何も移動しません。
delete-profile-and-backups = プロファイルを削除し、バックアップをごみ箱へ
delete-profile-body-backups = プロファイルとそのスケジュールされたタスクが削除され、バックアップはごみ箱に移動します。
delete-backups-running = 「{ $name }」のバックアップをごみ箱に移動しています…
delete-backups-done = 「{ $name }」のバックアップはごみ箱にあります。
delete-backups-left = 「{ $name }」のバックアップはごみ箱にあります。{ $path } には Archeion が書き込んでいない項目が { $count } 個あるため、残しました。
delete-backups-failed = 「{ $name }」のバックアップをすべてごみ箱に移動することはできませんでした: { $detail }
delete-backups-no-bin = { $path } にはごみ箱がありません (ネットワーク共有またはリムーバブル ドライブ)。「{ $name }」のバックアップには触れていません。不要なら手動で削除してください。

day-monday = 月曜日
day-tuesday = 火曜日
day-wednesday = 水曜日
day-thursday = 木曜日
day-friday = 金曜日
day-saturday = 土曜日
day-sunday = 日曜日

error-empty-name = プロファイルに名前を付けてください。
error-no-sources = バックアップするフォルダーを少なくとも 1 つ追加してください。
error-no-destination = バックアップの保存先を選んでください。
error-destination-in-source = 保存先がバックアップ対象のフォルダー内にあります。別の場所を選んでください。

# Backups tab
refresh = 更新
reading-backups = バックアップを読み込んでいます…
no-backups-yet = このプロファイルのバックアップはまだありません。
kind-complete = 完全
kind-incremental = 増分

# Status bar
ready = 準備完了
cancel = キャンセル
undo = 元に戻す
phase-starting = 開始しています…
phase-opening = 保存先を開いています…
phase-scanning = 変更を探しています…
phase-copying = コピーしています…
phase-merging = 古いバックアップを統合しています…
phase-restoring = 復元しています…
phase-verifying = 確認しています…
progress-of = { $done } / { $total }
skipped-count = { $count ->
   *[other] { $count } 個のファイルをスキップしました (使用中)
}
profile-removed = プロファイル「{ $name }」を削除しました。
run-finished = 「{ $name }」をバックアップしました: 新規 { $new }、変更 { $changed }、削除 { $deleted }、コピー { $added }。
merge-finished = 「{ $name }」を統合しました: バックアップ { $before } 個を { $after } 個に、{ $freed } を解放。
restore-finished = { $count } 個のファイルを復元しました。
verify-finished = { $count } 個のファイルを確認しました。問題: { $problems } 件。
run-destination-missing = 保存先が見つかりません: { $location }。ドライブを接続してもう一度お試しください。
run-busy = このプロファイルの別のバックアップまたは統合が実行中です。終了してからもう一度お試しください。
run-failed = 操作に失敗しました: { $detail }
run-no-space = 保存先の空き容量が足りません。必要: { $needed }、空き: { $available }。空き容量を増やすか、古いバックアップを統合または削除してから、もう一度実行してください。コピー済みのファイルは保持され、再度コピーされることはありません。
restore-no-space = 復元先の空き容量が足りません。必要: { $needed }、空き: { $available }。何も書き込まれていません。空き容量を増やすか、復元するファイルを減らすか、別のドライブに復元してください。
backup-resumed = { $count ->
   *[other] 中断されたバックアップから { $count } 個のファイルを引き継ぎました
}
backup-as-changes = { $count ->
   *[other] { $count } 個のファイルを変更分だけ保存しました
}
backup-efs-clear = { $count ->
    [one] Windows (EFS) で暗号化されたファイル 1 個を、その暗号化なしで保存しました
   *[other] Windows (EFS) で暗号化されたファイル { $count } 個を、その暗号化なしで保存しました
}
backup-efs-clear-hint = 保護したままにするには、このプロファイルの新しい保存先で暗号化を有効にしてください。
run-cancelled = キャンセルしました。バックアップは何も変更されていません。停止したバックアップは、次回、停止した位置から続行します。

# Errors
settings-unreadable = 設定ファイルを読み込めないため、変更は保存されません: { $detail }
save-failed = 設定を保存できませんでした: { $detail }
schedule-failed = 自動実行をスケジュールできませんでした: { $detail }
start-failed = 操作を開始できませんでした: { $detail }
unexpected-stop = 操作が予期せず停止しました。

# Restore browser
dates = バックアップの日付
archive-tooltip = { $files } 個のファイル、このバックアップで保存: { $stored } 個 ({ $size })
search-hint = すべてのバックアップからファイル名またはフォルダー名で検索…
include-deleted = チェックしたフォルダーに削除されたファイルを含める
include-deleted-hint = オン: チェックしたフォルダーは、その後元の場所で削除されたファイルも、名前の前に _deleted_ を付けて戻します。オフ: その日付の時点の状態で戻ります。削除されたファイルは個別にチェックすることもできます。
no-results = その名前のファイルまたはフォルダーはありません。
results = { $count ->
   *[other] { $count } 個のファイルが見つかりました
}
close = 閉じる
restore-to = 復元先:
target-beside = 元のファイルの隣 (日付付き)
target-beside-hint = report.docx は「report (2026-11-02 21.30).docx」として復元されます。現在のファイルには触れません。
target-original = 元の場所
target-folder = 別のフォルダー
replace-existing = 既存のファイルを置き換える
replace-warning = 現在のファイルがバックアップのコピーで置き換えられます。
tick-files-hint = ファイルまたはフォルダーを少なくとも 1 つチェックしてください。
choose-folder-hint = 復元先のフォルダーを選んでください。
restore-selected = 復元
clear-selection = 選択を解除
tick-folder = このフォルダーと中身をすべて復元
tick-file = このファイルを復元
backups-restore-files = ファイルを復元
backups-restore-files-hint = これまでに保存されたすべてのファイル: 戻したいものにチェックを付け、バージョンまたは日付と復元先を選びます。
backups-manage = バックアップの管理
backups-manage-hint = バックアップの日付の一覧: 複数を 1 つに統合したり、1 つを削除したりします。
restore-tree-count = バックアップ内のファイル { $files } 個 · 元の場所から削除: { $deleted }
restore-tree-hint = 復元するファイルやフォルダーにチェックを付けます。検索ではファイル名やフォルダー名で探せます (大文字と小文字は区別しません)。{ $deleted-icon } 削除済み の付いたファイルは元の場所にはもうありませんが、復元できます。
deleted-label = 削除済み
excluded-label = 除外
tree-excluded-file = バックアップ後に追加された除外で外れています。保存済みのバージョンは残ります。チェックしたフォルダーでは戻りません。単独でチェックすると、名前に日付を付けて戻ります。
tree-excluded-folder = バックアップ後に追加された除外で外れています。保存済みのバージョンは残ります。中身を一つずつチェックすると戻せます。
tick-excluded-in-folder = 後から追加された除外により、チェックしたフォルダー { $folder } から外れています。チェックすると、名前に日付を付けて一緒に復元されます。
tree-deleted-file = 最新のバックアップにありません: 元の場所から削除されています。チェックを付けると、そのバージョンの 1 つを復元できます。
tree-deleted-folder = 最新のバックアップにありません: 元の場所から削除されています。チェックを付けると、ある日付の状態で復元できます。
tick-in-folder = チェックしたフォルダー { $folder } と一緒に復元されます。除外するにはチェックを外してください。
tick-not-in-folder-date-folder = チェックしたフォルダー { $folder } の復元元のバックアップにないため、復元されません。これだけを選ぶには、そのフォルダーのチェックを外してください。
tick-not-in-folder-date = チェックしたフォルダー { $folder } の復元元のバックアップにはありません (それより前に元の場所で削除されました)。チェックを付けると、最後のバージョンまたは選んだバージョンでこれも復元されます。
tick-partial = 中にチェックした項目、またはチェックしたフォルダーから除外した項目があります。クリックするとフォルダー全体をチェックします。
tick-full = 中の項目がすべて個別にチェックされています。クリックするとすべてのチェックを外します。
tick-left-out = チェックしたフォルダー { $folder } から除外されています。復元されません。フォルダーと一緒に復元するにはチェックしてください。
selection-empty = まだ復元する項目がありません。ツリーでファイルまたはフォルダーをチェックしてください。
file-name-hint = クリックするとバージョンが表示され、復元するものを選べます。
versions-hint = 復元するバージョンを選びます。選ばない場合は最新のものです。
version-detail = { $backup } のバックアップ · { $size }
version-modified = { $date } に変更
versions-in-folder = チェックしたフォルダー { $folder } と一緒に、そのバックアップの時点の状態で戻ります。別のバージョンを選ぶと、このファイルだけ個別に復元されます。
versions-tick-hint = 選んだバージョンを復元するには、ツリーでファイルにチェックを付けてください。
restore-to-hint = チェックしたすべての項目の復元先です。
restore-conflict = チェックした 2 つの項目が同じ元のファイルを置き換えます。どちらかのチェックを外してください。
restore-conflict-hint = チェックした 2 つの項目が同じ元のファイルを置き換えます。どちらかのチェックを外してください。
restore-wait-versions = チェックしたファイルのバージョンを読み込んでいます…
restore-summary = ファイル { $files } 個、フォルダー { $folders } 個 — { $size }

# Manual merge
merge-choose = 統合するバックアップを選ぶ…
merge-choose-hint = 最初と最後のバックアップにチェックを付けます。その間のすべてが 1 つのバックアップになります。
merge-tick = 統合する範囲の端としてチェック
merge-tick-hint = 統合する最も古いバックアップと最も新しいバックアップにチェックを付けます。その間のバックアップも含まれます。
merge-summary = { $count } 個のバックアップが、{ $date } 付けの 1 つになります。
merge-complete-warning = 範囲に最初の完全バックアップが含まれています。それは、選んだ中で最も新しい日付の新しい完全バックアップに置き換えられ、それより前の日付は復元できなくなります。
merge-complete-ack = 理解しました
merge-two-hint = バックアップを少なくとも 2 つチェックしてください。
merge-ack-hint = 警告を理解したことを確認してください。
merge-selected = 統合

# Activity tab
tab-activity = アクティビティ
show-details = 詳細
cancel-hint = 今すぐ停止します。バックアップはこの実行前の状態のまま残ります。次回のバックアップでは、コピー済みのファイルを再利用します。
activity-backup-running = 「{ $name }」をバックアップしています
activity-merge-running = 「{ $name }」のバックアップを統合しています
activity-restore-running = 「{ $name }」から復元しています
activity-preparing = 準備しています…
activity-file = ファイル
activity-folder = フォルダー
activity-files = ファイル数
activity-speed = 速度
activity-elapsed = 経過時間
activity-remaining = 残り時間
count-of = { $done } / { $total }
files-found = これまでに { $count } 個
about = 約 { $time }
activity-took = 所要時間: { $time }。
activity-backup-summary = 完了: 新規 { $new }、変更 { $changed }、削除 { $deleted }、変更なし { $unchanged }、コピー { $copied }。
activity-merge-summary = 完了: バックアップ { $before } 個を { $after } 個に、{ $freed } を解放。
activity-restore-summary = 完了: { $restored } 個のファイルを復元、{ $kept } 個は既存のため保持。
skipped-hint = これらのファイルは読み込めませんでした。通常は、別のプログラムが開いているためです。以前のバージョンがあれば、引き続きバックアップに含まれています。
skipped-more = …ほか { $count } 個 (ログを参照)。

# Exclusions and format lock
no-exclusions = 除外なし: フォルダー内のすべてのファイルをバックアップします。
remove-exclusion = 再びバックアップする
exclusion-folder = フォルダーとその中身すべて
exclusion-file = ファイル
exclusion-type = この種類のすべてのファイル
exclusion-pattern = パターン
exclude-folder = フォルダーを除外…
exclude-file = ファイルを除外…
exclude-type = 種類を除外
exclude-pattern = パターンを除外
exclude-rule-hint = **/node_modules · .iso
rule-empty = パターンかファイルの種類を入力し、どちらかを示すボタンを押してください。
rule-type-not-pattern = 「{ $text }」はファイルの種類です。「種類を除外」を押してください。
rule-pattern-not-type = 「{ $text }」はパターンです。「パターンを除外」を押してください。
rule-name-alone = パターンはパス全体と照合されるため、「{ $text }」だけでは何も除外されません。どこにあってもこの名前のフォルダーやファイルを除外するには、**/{ $text } と入力してください。
rule-bad-type = 「{ $text }」はファイルの種類ではありません。拡張子だけを空白なしで入力してください（例: .iso）。
rule-bad-pattern = 「{ $text }」は有効なパターンではありません: { $detail }。
rule-already = この除外はすでに一覧にあります。
container-locked = 保存先にはすでにこの形式のバックアップがあるため、形式は変更できません。別の形式を使うには、空の保存先を選んでください。
container-unreachable = 保存先にアクセスできないため、今は形式を変更できません。
container-checking = 保存先を確認しています…
activity-idle-title = 実行中の操作はありません
activity-idle-body = バックアップ、統合、復元の進行状況はここに表示されます。

# Backup kinds and deletion
menu-run-incremental = 今すぐ増分バックアップ
menu-run-complete = 今すぐ完全バックアップ
run-now = 今すぐ実行:
run-incremental = 増分
run-incremental-hint = 新規および変更されたファイルだけをコピーします。速く、通常はこちらを選びます。
run-complete = 完全
run-complete-hint = すべてのファイルをコピーし、新しいチェーンを開始します。以降のバックアップは以前のバックアップに依存しなくなります。時間と容量が多く必要です。
run-merge = 統合
run-first = 最初のバックアップを実行
run-incremental-button = 増分バックアップを実行
run-complete-button = 完全バックアップを実行
full-backup = 完全バックアップ:
full-never = 最初の 1 回のみ
full-every-backups = 次の回数ごと
full-every-days = 前回から次の日数が過ぎたとき
full-backups-unit = 回
full-days-unit = 日
full-backup-hint = 自動バックアップに適用されます。完全バックアップは新しいチェーンを開始するため、復元が古いバックアップに依存しなくなります。
run-finished-complete = 「{ $name }」を完全バックアップしました: 新規 { $new }、変更 { $changed }、削除 { $deleted }、コピー { $added }。
activity-complete-summary = 完全バックアップ完了: 新規 { $new }、変更 { $changed }、削除 { $deleted }、変更なし { $unchanged }、コピー { $copied }。
activity-delete-running = 「{ $name }」のバックアップを削除しています
activity-delete-summary = 完了: バックアップ { $before } 個を { $after } 個に、{ $freed } を解放。
delete-finished = 「{ $name }」のバックアップを 1 つ削除しました。
delete-backup = このバックアップを削除…
delete-backup-hint = 選択した日付を削除します。それ以降のバックアップは完全なまま残ります。
delete-backup-confirm = バックアップを削除
delete-initial-hint = 最初の完全バックアップは、それが唯一のバックアップの場合にのみ削除できます。他のバックアップはそれに依存しています。
delete-complete-hint = 完全バックアップは、以降のバックアップが依存するチェーンの起点のため削除できません。代わりに、後続のバックアップと統合してください。
delete-only-warning = { $date } のバックアップは唯一のバックアップです。保存先は空になり、次回は完全バックアップになります。この操作は元に戻せません。
delete-newest-warning = { $date } のバックアップを、そこにだけ保存されたファイルのバージョンとともに削除します。この操作は元に戻せません。
delete-middle-warning = { $date } のバックアップを削除します。以降のバックアップに必要なファイルは次のバックアップに移され、このバックアップにだけ保存されたバージョンは失われます。この操作は元に戻せません。

# Encryption
encrypt = バックアップを暗号化 (AES-256、ファイル名を含む)
encrypt-hint = 各ファイルは名前を隠した AES-256 の zip になります。パスワードがなければ、誰も内容を読むことも、どのファイルが保存されているかを見ることもできません。パスワードは Windows 資格情報マネージャーに保管されます。
encrypt-needs-zip = 暗号化には「ファイルごとに zip」形式が必要です。
password = パスワード
password-again = パスワードの確認
password-set = パスワードを設定
password-reenter = これらのバックアップのパスワードを入力
password-saved = パスワードを Windows 資格情報マネージャーに保存しました。
password-missing = このコンピューターにはパスワードが保存されていません。バックアップの実行や閲覧には、パスワードを入力してください。
password-loss-warning = このパスワード、または最初のバックアップ後に表示される回復キーがなければ、バックアップは開けず、誰にも回復できません。パスワードは安全な場所に保管してください。
password-save = パスワードを保存
password-checking = パスワードを確認しています…
password-empty-hint = パスワードを入力してください。
password-short-hint = { $count } 文字から { $max } 文字で入力してください。
password-mismatch-hint = 2 つのパスワードが一致しません。
password-rule-length = { $count } 〜 { $max } 文字 (現在 { $typed } 文字)
password-rule-match = 2 つのパスワードが一致している
password-rule-common = 「Password2024」や「qwerty」のようなよくあるパスワードではない
password-rule-runs = 「aaaa」「1234」「abcd」のような { $count } 文字の繰り返しや連続がない
password-rule-name = プロファイル名を含まない
password-rule-strength = 推測されにくい: 強度が「{ $level }」以上
password-rule-tip = 空白を含め、どの文字でも使えます。4〜5 語の文は覚えやすく、推測されにくいものです。数字、大文字、記号は必要ありません。
password-strength = 強度: { $level }
password-strength-weak = 弱い
password-strength-fair = 普通
password-strength-strong = 強い
password-strength-very-strong = 非常に強い
password-common-hint = これは最もよく使われているパスワードの 1 つです。別のパスワードを選んでください。
password-runs-hint = 「aaaa」や「1234」のような並びは避けてください。
password-name-hint = パスワードにプロファイル名を含めないでください。
password-weak-hint = 推測されやすすぎます。たとえば数語を足して、長くしてください。
password-first-hint = 上で暗号化バックアップのパスワードを入力してください (新しいバックアップでは 2 回入力し、「理解しました」にチェック)。ボタンを押すとパスワードを保存してバックアップを開始します。
password-wrong = このパスワードでは保存先のバックアップを開けません。
password-unreachable = パスワードを確認できませんでした
run-password-needed = バックアップは暗号化されていますが、このコンピューターにはパスワードが保存されていません。プロファイルの設定で入力してください。
run-wrong-password = 保存されたパスワードではバックアップを開けません。プロファイルの設定でもう一度入力してください。

# Operations log
log-runs = 操作
log-only-profile = このプロファイルのみ
log-running = 実行中
log-empty = 操作はまだありません。
log-pick = 左側で操作を選んでください。
log-title = ログ
log-no-lines = まだ何も記録されていません。
log-scheduled = 自動
log-manual = 手動で開始
log-started = 開始: { $time }
log-backup = バックアップ { $name }
log-unfinished = 完了していません: 中断されたか、まだ実行中です。
log-skipped = スキップ: { $path }: { $reason }
log-op-backup = バックアップ
log-op-incremental = 増分バックアップ
log-op-complete = 完全バックアップ
log-op-merge = 統合
log-op-delete = バックアップの削除
log-op-restore = 復元
log-open-full = ログ全体を開く
log-open-full-hint = この操作のログ全体をテキスト ファイルに書き出し、テキスト エディターで開きます。各手順に続いて、処理したすべてのファイルとその処理内容 (スキップしたファイルを含む) が記録されます。
log-copy = ログをコピー
log-copy-hint = この操作のログ全体をテキストとしてコピーします。メールや問題の報告に貼り付けられます。
log-copied = ログをコピーしました。
log-open-program = プログラムのログ
log-open-program-hint = プログラムの技術ログ archeion.log を開きます。問題の調査や報告に役立ちます。
log-open-folder = ログ フォルダー
log-open-folder-hint = 操作ログのフォルダーをエクスプローラーで開きます。
log-program-missing = プログラムのログはまだありません。
log-unreadable = ログ ファイルを読み込めません。
log-report-failed = ログ全体を表示できません: { $detail }
log-level-info = 情報
log-level-done = OK
log-level-warning = 警告
log-level-error = エラー
log-preparing = ログ全体を準備しています…
log-files-title = ファイル
log-files-missing = この操作ではファイルの一覧が記録されていません。以前のバージョンの Archeion で実行されました。
log-files-none = 処理したファイルはありません。
log-file-new-chunks = 変更として保存: 新しいチャンクを { $size } 書き込み
log-file-resumed = 中断したバックアップでコピー済み、そのまま保持
log-count-new = 新規: { $count }
log-count-changed = 変更: { $count }
log-count-as-changes = 変更として保存: { $count }
log-count-copied-again = 再コピー: { $count }
log-count-unchanged = 変更なし (一覧には含めない): { $count }
log-count-deleted = 削除: { $count }
log-count-moved = 移動: { $count }
log-count-dropped = 削除した旧バージョン: { $count }
log-count-restored = 復元: { $count }
log-count-kept = 保持: { $count }
log-count-checked = 読み戻して問題なし: { $count }
log-count-checked-unlisted = 読み戻して問題なし (一覧には含めない): { $count }
log-count-damaged = 問題: { $count }
log-count-skipped = スキップ: { $count }

# Password change and recovery key
password-change = パスワードの変更
password-new = 新しいパスワード
password-change-button = パスワードを変更
recovery-later = 回復キーは最初のバックアップの後に利用できます。
recovery-show = 回復キーを表示…
recovery-title = 回復キー
recovery-hint = このキーがあれば、Archeion やパスワードがなくても、7-Zip ですべての暗号化ファイルとマニフェストを開けます。各バックアップのマニフェストには、どのファイルがどこに保存されているかが記載されています。
recovery-warning = このキーを持つ人は誰でもすべてのバックアップを読めます。印刷するか書き留めて、オフラインで保管してください。パスワードを変更してもキーは変わりません。
recovery-copy = コピー
recovery-copy-hint = 7-Zip が求める形式で、スペースなしでキーをコピーします。
recovery-hide = 隠す
password-current = 現在のパスワード
password-current-hint = 現在のパスワードを入力してください。
recovery-ask = 回復キーを表示するには、これらのバックアップの現在のパスワードを入力してください。
recovery-show-button = 表示

# Integrity check
verify-section = 確認
verify-hint = 完全確認では、すべてのバックアップに保存された各ファイルを読み直し、フィンガープリントと照合します。復元テストでは、最新のバックアップからいくつかのファイルを無作為に読み直します。期限が来た確認は、スケジュールされたバックアップの後に実行されます。
verify-full-every = 完全確認の間隔
verify-sample-every = 復元テストの間隔
verify-sample-files = 対象ファイル数:
verify-zero-hint = 0 日 = 自動では行わない。
verify-full-now = 今すぐバックアップを確認
verify-full-hint = 保存された各ファイルを読み直して確認します。保存先が大きい場合、すべてのバックアップを読むのと同じだけ時間がかかります。
verify-sample-now = 今すぐ復元テスト
verify-sample-hint = 復元と同じように最新のバックアップからいくつかのファイルを無作為に読み直します。どこにも書き込みません。
verify-no-backups = 確認するバックアップはまだありません。
verify-problems = 問題が見つかりました
verify-problems-hint = 一覧のバックアップは正しく復元できない可能性があります。今すぐ完全バックアップを行い、保存先のドライブを確認してください。
activity-verify-running = 「{ $name }」のバックアップを確認しています
log-op-verify = 整合性の確認
log-op-restore-test = 復元テスト

# Automatic backup: schedules, events, notifications
schedule-days = 選んだ曜日
schedule-hours = 数時間ごと
schedule-monthly = 月に 1 回
schedule-all-days = 毎日
schedule-add-time = + 時刻を追加
schedule-remove-time = この時刻を削除
schedule-every = 間隔
hours-unit = 時間
schedule-from = 開始
schedule-until = 終了
schedule-hours-hint = 最後の実行は終了時刻より前に開始されます。開始時刻と終了時刻が同じ場合は 24 時間を意味します。
schedule-month-day = 日付
schedule-month-hint = どの月にもある日にするため、1 日から 28 日のみ選べます。
alerts = 通知
alert-on-failure = 自動バックアップが失敗したら通知する
alert-stale = 次の期間バックアップがなければ警告する
alert-hint = Windows の通知です。0 日 = 警告しない。確認は毎日とサインイン後に行われます。
alert-manual-hint = 自動バックアップのみが対象です。「自動バックアップ」でスケジュールを選んでください。
alert-failed-title = Archeion: 「{ $name }」のバックアップに失敗しました
alert-stale-title = Archeion: 「{ $name }」がバックアップされていません
alert-stale-body = { $days } 日間バックアップがありません (前回: { $when })。保存先が接続されているか確認するか、Archeion からバックアップを開始してください。
alert-unusual-title = Archeion: 「{ $name }」に通常と異なる変更
alert-unusual-body = 前回のバックアップ以降、{ $count } 個のファイルが新規、変更、または削除されており、通常よりはるかに多くなっています。心当たりがない場合はファイルを確認してください。以前のバージョンはバックアップに残っています。
alert-went-back-title = Archeion: 「{ $name }」の保存先のバックアップが古くなっています
alert-went-back-body = 以前そこで確認した最新のバックアップがなくなっています。保存先が古いコピーから戻されたか、Archeion の外でバックアップが削除されました。心当たりがない場合は保存先を確認してください。
alert-changed-outside-title = Archeion: 「{ $name }」の保存先のバックアップが変更されています
alert-changed-outside-body = Archeion の外で保存先のバックアップが削除されたか、戻されました。バックアップの一覧が Archeion の署名した一覧と一致しません。心当たりがない場合は保存先を確認してください。
run-changed-outside = Archeion の外で保存先のバックアップが削除されたか、戻されました。心当たりがない場合は保存先を確認してください
closing-after-run = 操作が実行中です。終了すると Archeion は自動的に閉じます。
closing-cancel = 閉じない
shutdown-reason = バックアップ操作を実行中です。今 Windows をシャットダウンすると中断され、次回に続きから行われます。

# Notification area and Preferences window
menu-preferences = 環境設定…
preferences-title = 環境設定
preferences-window = ウィンドウ
language = 言語
language-system = システムの言語
language-tray-restart = 通知領域のアイコンのメニューは、次回 Archeion を起動したときに言語が切り替わります。
quit-hint = Archeion を終了します。スケジュールされたバックアップは引き続き Windows タスク スケジューラから実行されます。
already-open-title = Archeion はすでに開いています
already-open-other =
    Archeion は別のコピーからすでに開いています: { $path }

    そのウィンドウを前面に表示しました。このコピーを使うには、[ファイル] > [終了] でそちらを閉じてから、もう一度このコピーを起動してください。
tray-open = Archeion を開く
tray-quit = 終了
tray-running = 操作を実行中
option-tray = ウィンドウを閉じても通知領域で実行を続ける
option-tray-hint = ウィンドウを閉じると時計の横に隠れ、Archeion は古いバックアップの統合と実行中の操作を続けます。アイコンをクリックすると再び開きます。完全に終了するには「終了」を選んでください。
option-tray-unavailable = Windows から通知領域のアイコンが提供されませんでした。
option-autostart = Windows の起動時に Archeion を開始
option-autostart-hint = サインインすると、Archeion は通知領域に隠れた状態で起動します。タスク マネージャーの「スタートアップ アプリ」でオフにすることもできます。
autostart-failed = Windows の起動時に開始する設定を変更できませんでした: { $detail }

# Help menu: user guide and About
menu-documentation = ドキュメント
menu-about = Archeion について
about-version = バージョン { $version }
about-name = 名前は古代ギリシャ語の ἀρχεῖον (「記録保管所」) に由来します。
about-author = 作者: MR
about-license = ライセンス: GPL-3.0
about-repository = GitHub のソース コード
about-logs-folder = ログ フォルダー
about-open-folder = フォルダーを開く
about-folder-missing = フォルダーはまだありません。
doc-title = Archeion のドキュメント
doc-intro = Archeion はフォルダーのコピーを別のドライブ、ネットワーク共有、その他任意のフォルダーに保存し、どのファイルでも過去の日付の状態で取り戻せるようにします。セクションを開いてお読みください。
doc-what-title = Archeion の機能
doc-what-backups = プロファイルの最初のバックアップは完全バックアップで、すべてのファイルをコピーします。以降は増分バックアップで、新規および変更されたファイルだけをコピーするため速くて小さく、それでいてどの日付も丸ごと復元できます。プロファイルの設定の「完全バックアップ」では、一定回数のバックアップごと、または前回から一定の日数が過ぎたときに、新しい完全バックアップを行うようにも設定できます。完全バックアップは新しいチェーンを開始し、古いバックアップに依存しなくなります。
doc-what-storage = 各ファイルは元と同じフォルダー構成で個別に保存されます。保存形式は、そのファイルの zip (「ファイルごとに zip」: report.docx は report.docx.zip になります)、そのままのコピー、AES-256 で暗号化した zip のいずれかです。バックアップは Archeion なしでも読めます。zip はエクスプローラーや 7-Zip で開け、暗号化された zip は回復キーを使って 7-Zip で開けます。
doc-what-safe = バックアップが中断されても (コンピューターの電源が切れた、ドライブが取り外された、キャンセルした)、既存のバックアップはそのまま残ります。次回のバックアップは停止した位置から続行し、コピー済みのファイルを再度コピーすることはありません。
doc-profiles-title = プロファイル
doc-profiles-body = プロファイルでは、何を、どこに、いつバックアップするかを決めます。「ファイル」>「新しいプロファイル」(Ctrl+N) で作成し、「設定」タブに名前、バックアップするフォルダー、保存先 (フォルダー、外付けドライブ、ネットワーク共有のいずれか) を入力します。保存先のフォルダーは 1 つのプロファイル専用にしてください。
doc-profiles-list = プロファイルの一覧は、あなたが決めた順序を保ちます。プロファイルを上下にドラッグするか、「上へ移動」と「下へ移動」(Alt+↑、Alt+↓) を使います。プロファイルを右クリックするとそのコマンドが開きます (ファイル メニューにもあります)。「名前の変更」(F2) はその場で名前を変え、Enter で確定、Esc で取り消します。「プロファイルを削除…」(Delete) は確認を求めます。
doc-profiles-duplicate = 「複製」(Ctrl+D) は、保存先以外のすべての規則を持つプロファイルのコピーをその下に追加します。保存先はあなたが選び、スケジュールはあなたが有効にするまで待機します。
doc-profiles-exclusions = 除外は不要なものを外します。フォルダーとその中身、ファイル、.iso のような種類のすべてのファイル（「種類を除外」）、またはパターンに一致するもの（「パターンを除外」）です。パターンは大文字と小文字を区別せずにパス全体と照合され、* は任意の文字列、** は任意のフォルダー、? は 1 文字です（例: **/node_modules、**/cache/**）。欄にルールを入力し、それが何かを示すボタンを押すと、テキストが確認されます。新しいプロファイルでは .tmp、.temp と tmp・temp フォルダーが最初から除外されます。バックアップ後に除外したファイルは保存済みのバージョンを残し、フォルダーを復元しても戻りませんが、単独でチェックすると名前に日付を付けて戻ります。
doc-profiles-format = 保存形式 (ファイルごとに zip、またはそのままコピー) と暗号化は、最初のバックアップの前に選ぶ必要があります。保存先にバックアップができると変更できません。切り替えるには、空の保存先を選んでください。
doc-profiles-delete = 「プロファイルを削除」は確認したうえで、プロファイルとそのスケジュールされたタスクを削除します。バックアップは保存先に残ります。削除の直後なら、ステータス バーの「元に戻す」でプロファイルを復元できます。後でそのフォルダーを使うプロファイルは、「これらのバックアップをこのプロファイルで使う」でバックアップを引き継げます。確認画面で [バックアップもごみ箱に移動する] を選ぶと、バックアップもごみ箱に移動し、プロファイルは元に戻せません。移動するのは Archeion がそのフォルダーに書き込んだものだけで、ほかに何もなければフォルダーごと移動します。ネットワーク共有やリムーバブル ドライブにはごみ箱がないため、そこでは何も移動しません。
doc-auto-title = 自動バックアップ
doc-auto-schedule = 「自動バックアップ」では、バックアップが自動で実行されるタイミングを選びます。選んだ曜日の 1 つ以上の時刻、2 つの時刻の間で数時間ごと、または毎月 1 日から 28 日のいずれかの日に 1 回です。Archeion はスケジュールを Windows タスク スケジューラに登録するため、Archeion を開いていなくてもバックアップは実行されます。
doc-auto-missed = 予定の時刻にコンピューターの電源が切れていた場合は、次に起動したときすぐにバックアップを実行します。
doc-auto-alerts = 「通知」では、Archeion は自動バックアップが失敗したときに Windows の通知を表示したり、プロファイルのバックアップが一定の日数行われていないときに警告を表示したりできます (確認は毎日とサインイン後に行われます)。
doc-auto-now = バックアップはいつでも手動で開始できます。プロファイルの設定の「今すぐバックアップ」か、「バックアップ」>「今すぐ増分バックアップ」(Ctrl+B) と「今すぐ完全バックアップ」を使います。
doc-merge-title = 古いバックアップの統合
doc-merge-body = 古いバックアップは削除されず、次に残すバックアップに統合されます。残した日付はすべて完全なまま、不要になったバージョンの容量が解放されます。統合は自動で行われます。Archeion の実行中に行われ、バックアップや他の操作が保存先を使用している間は行われません。統合後のバックアップには、統合するバックアップの間に元の場所で削除されたファイルが最後の内容のまま残ります。[元の場所で削除されたファイルを統合後のバックアップに残す] のチェックを外すと残りません。自動の統合はプロファイルの設定で、手動の統合は [バックアップの管理] で選べます。
doc-merge-switch = 「古いバックアップを自動的に結合する」で自動結合をオンまたはオフにします。オフの場合、[バックアップ] > [バックアップの管理] で結合または削除するまで各バックアップは残り、「今すぐ結合」は表示されません。規則はそのまま残ります。バックアップを結合する規則で再びオンにすると、あなたの確認を待ちます。
doc-merge-rules = 「古いバックアップの統合」のルールで、どのバックアップを残すかを決めます。いずれかのルールで選ばれたバックアップは残されます。直近の日数分のすべてのバックアップ、最新のもの、1 日に 1 つ、1 週間に 1 つ、1 か月に 1 つ、1 年に 1 つを、それぞれ設定した数だけ残します。すべてのルールが 0 の場合、すべてのバックアップを残し、何も統合しません。
doc-merge-confirm = 以前の規則で残っていたバックアップを結合する規則は、自動では適用されません。「新しい規則で結合」か「以前の規則に戻す」を選ぶまで、自動結合は待機します (「今すぐ結合」は同意と見なされます)。規則の下の行に、次の結合で何が結合されるかが表示されます。
doc-merge-first = 最初の完全バックアップが自動で統合されることはありません。
doc-merge-manual = 「バックアップ」>「バックアップの管理」の「統合するバックアップを選ぶ…」で手動で統合できます。最も古いバックアップと最も新しいバックアップにチェックを付けると、その間のすべてが 1 つのバックアップになります。範囲に最初の完全バックアップが含まれる場合、それより前の日付は復元できなくなるため、Archeion が先に警告します。
doc-merge-delete = 「バックアップ」>「バックアップの管理」の「このバックアップを削除…」は 1 つの日付を削除します。それ以降のバックアップは完全なまま残ります。以降のバックアップが依存している完全バックアップは削除できません (代わりに統合してください)。削除は元に戻せません。
doc-encrypt-title = 暗号化
doc-encrypt-body = 「バックアップを暗号化」を有効にすると、各ファイルは名前を隠した AES-256 の zip になります。パスワードがなければ、誰も内容を読むことも、どのファイルが保存されているかを見ることもできません。暗号化には「ファイルごとに zip」形式が必要で、最初のバックアップの前に有効にする必要があります。後から変更することはできません。
doc-encrypt-password = パスワードは Windows 資格情報マネージャーに保存されるため、自動バックアップで入力する必要はありません。後からプロファイルの設定で、現在のパスワードを入力して変更できます。別のコンピューターでは、プロファイルの設定で一度入力してください。
doc-encrypt-recovery = 最初のバックアップの後、「回復キーを表示…」でキーが表示されます。このキーがあれば、Archeion やパスワードがなくても、7-Zip ですべての暗号化ファイルを開けます。印刷するか書き留めて、オフラインで保管してください。パスワードもキーもなければ、誰もバックアップを開けません。パスワードを変更してもキーは変わりません。キーを表示する前に、Archeion は現在のパスワードを求めます。
doc-large-title = 大きなファイル
doc-large-body = 少しずつ変わる大きなファイル (メールボックス、仮想マシンのディスク、データベースなど) は、毎回まるごとコピーされません。約 1 MB の部分に分割して保存され、各バックアップには新しい部分だけが保存されます。同じ部分は一度だけ保存されます。対象は、プロファイルの「保存形式」セクションで選んだサイズ以上のファイル (既定は 500 MB、無効にもできます) と、暗号化された保存先のすべてのファイルです。
doc-large-complete = 完全バックアップには引き続きすべてのファイルが zip、通常のコピー、または暗号化 zip として丸ごと保存されるため、7-Zip だけで復元できます。分割された部分を含むのは、その後の増分バックアップだけです。復元では部分を組み立て直し、それぞれを検査します。整合性チェックはすべての部分を読み取ります。
doc-large-upgrade = 以前のバージョンで作成したバックアップはそのまま残り、引き続き読み取れます。分割保存は次の完全バックアップから始まります。そのような保存先では、次の自動バックアップが完全バックアップになります。
doc-restore-title = ファイルの復元
doc-restore-body = プロファイルの「バックアップ」タブ (または「バックアップ」>「ファイルを復元…」) を開きます。「ファイルを復元」には、バックアップがこれまでに保存したすべてのファイルとフォルダーが 1 つのツリーで表示されます。元の場所にもうないファイルには { $deleted-icon } 削除済み が付きますが、復元できます。ツリーまたは検索結果でファイルやフォルダーにチェックを付けます (検索ボックスではファイル名やフォルダー名で探せます。大文字と小文字は区別しません)。
doc-restore-versions = チェックできるのはチェック ボックスだけです。ファイル名をクリックすると、そのバージョンが新しい順に右側に表示されます。選んだバージョンが復元され、選ばなければ最新のものが復元されます。チェックしたフォルダーは、それを含む最新のバックアップの時点の状態ですべて戻ります。フォルダー名をクリックしても何も起きません。上下の矢印キーで上や下のファイルに移動できます。チェックしたフォルダー内のファイルは、別のバージョンを選ばない限りフォルダーの日付の状態で戻ります。チェックしたフォルダーは、以前に含まれていて、その後元の場所で削除されたファイルも、それぞれ最後のバージョンで戻します。元の場所にもうないファイルは、復元時に名前の前に「_deleted_」が付きます (_deleted_report.docx)。
doc-restore-where = 右側のパネルの上部で、すべての復元先を選びます。元のファイルの隣に日付付きの名前で (report.docx は「report (2026-11-02 21.30).docx」として戻り、現在のファイルには触れません)、元の場所、または別のフォルダーです。既存のファイルは、「既存のファイルを置き換える」にチェックを付けた場合にのみ置き換えられます。「復元」を押すと、すべてが 1 回の操作で戻ります。隣の [選択を解除] はすべてのチェックを外します。
doc-verify-title = 整合性の確認と復元テスト
doc-verify-body = 完全確認では、すべてのバックアップに保存された各ファイルを読み直し、フィンガープリントと照合します。復元テストでは、復元と同じように最新のバックアップからいくつかのファイルを無作為に読み直し、どこにも書き込みません。プロファイルの設定の「確認」セクション (または「バックアップ」>「今すぐバックアップを確認」) から開始するか、スケジュールされたバックアップの後に数日ごとに実行されるように設定します。問題が見つかった場合は、完全バックアップを行い、保存先のドライブを確認してください。
doc-activity-title = 「アクティビティ」タブ
doc-activity-body = 「アクティビティ」タブには、実行中のバックアップ、統合、復元、確認の進行状況が「キャンセル」ボタンとともに表示され、自動または手動で開始した過去のすべての操作のログも表示されます。ログには、操作の内容と、別のプログラムが開いていたためにスキップされたファイルが記録されます。操作の手順の上にある「ログ全体を開く」は、ログ全体をテキスト ファイルとしてエディターで表示します。手順に続いて、処理したすべてのファイルが、時刻、処理内容 (新規、変更、変更として保存、再コピー、削除、移動、旧バージョンを削除、復元、保持、スキップ、問題)、サイズ、フル パスとともに記録されます。増分バックアップの変更のないファイルと、完全な確認で問題のなかったファイルは数だけが示されます。「ログをコピー」は同じテキストをコピーします。「プログラムのログ」は、より込み入った問題のために Archeion の技術ログを開きます。
doc-window-title = ウィンドウを閉じる
doc-window-body = ウィンドウを閉じても Archeion は停止しません。時計の横の通知領域に移って動作を続けるため、古いバックアップの統合も続きます。ウィンドウは、そこにあるアイコンから再び開けます。Archeion は Windows の起動時にも、そこに隠れた状態で起動します。「ファイル」>「終了」、またはアイコンのメニューの「終了」で、完全に終了します。どちらの動作も「ファイル」>「環境設定」でオフにでき、そこで言語も選べます。スケジュールされたバックアップは、Archeion が実行されていなくても必ず実行されます。
doc-cli-title = コマンド ライン
doc-cli-body = 引数なしの archeion.exe はウィンドウを開きます。次のコマンドは、スケジュールされたタスクと同じように、ウィンドウなしで 1 つの操作を実行します。プロファイル ID は、settings.toml にあるプロファイルの id です。
doc-cli-run = プロファイルをバックアップします (--kind complete で完全バックアップ)。
doc-cli-merge = プロファイルのルールに従って古いバックアップを統合します。
doc-cli-verify = バックアップを確認します (--sample N: N 個のファイルで復元テスト)。
doc-cli-delete = バックアップを 1 つ削除します。
doc-cli-restore = ウィンドウが書き出した要求ファイルに記載されたファイルを復元します。
doc-cli-watch = 最近バックアップされていないプロファイルについて警告します。タスク スケジューラが毎日実行します。
doc-cli-extract = 設定やプロファイルなしで、バックアップ フォルダーだけから復元します。たとえば別のコンピューターでポータブル版の Archeion を使う場合です。ファイルは復元先フォルダーの下に完全なパスで置かれます。
doc-cli-extract-options = extract のオプション: --archive <名前> でバックアップを選びます (省略時は最新)。--path <ファイルまたはフォルダー> では、元のパスで指定したものだけを復元します。--overwrite は既存のファイルを置き換えます。暗号化されたバックアップでは、パスワードまたは回復キーを --stdin (標準入力の 1 行目)、環境変数 ARCHEION_PASSWORD または ARCHEION_RECOVERY_KEYで指定します。進行状況は復元先フォルダーの archeion-extract.log にも書き込まれます。コマンド プロンプトでは、終了を待つように start /wait archeion extract … として実行してください。
doc-cli-exit = 終了コード: 0 成功、1 失敗、2 保存先が見つからない、3 使用中 (別の操作が保存先を使用中)、4 パスワードがないか誤り、5 空き容量不足。
doc-data-title = データとログの場所
doc-data-body = 設定は %APPDATA%\Archeion\config フォルダー (settings.toml) に、各操作のログはその logs サブフォルダーにあります。ポータブル版 (archeion.exe の隣に portable.txt ファイルがあるもの) は、代わりに実行ファイルの隣の data フォルダーにすべてを保存します。「ヘルプ」>「Archeion について」でこれらのフォルダーを開けます。
doc-data-backups = バックアップそのものは保存先にだけあります。バックアップごとに日付と時刻の名前が付いたフォルダーがあり、保存されたファイルと、その時点のすべてのファイルの一覧が入っています。

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
   *[other] { $count } 個のファイルが見つかりました
}
step-folders = { $count ->
   *[other] { $count } 個のフォルダーを読み込みました
}
step-new = 新規 { $count }
step-changed = 変更 { $count }
step-unchanged = 変更なし { $count }
step-copied-again = 変更なしで再コピー { $count }
step-moved = { $count ->
   *[other] { $count } 個のファイルを移動しました
}
step-restored = 復元 { $count }
step-kept = 既存のため保持 { $count }
step-checked = 確認済み { $count }
step-damaged = { $count ->
   *[other] 問題 { $count } 件
}
step-skipped = スキップ { $count }
merge-group = { $count } 個のバックアップ ({ $first } から { $last }) を 1 つに
merge-group-of = グループ { $index }/{ $of }: { $count } 個のバックアップ ({ $first } から { $last }) を 1 つに
recent-title = 最近のファイル
recent-new = 新規
recent-changed = 変更
recent-copied-again = 再コピー
recent-moved = 移動
recent-restored = 復元
recent-kept = 保持
recent-checked = OK
recent-damaged = 問題
recent-skipped = スキップ
recent-deleted = 削除
recent-dropped = 旧バージョンを削除

# Profile files (File menu)
import-profile = プロファイルをインポート…
export-profile = プロファイルをエクスポート…
import-profile-hint = Archeion でエクスポートしたファイルからプロファイルを追加します（ルールのみで、パスワードは含まれません）。
export-profile-hint = このプロファイルのすべてのルールをファイルに保存し、保管したり別のコンピューターへ移したりできます。パスワードは保存されません。
profile-file-type = Archeion プロファイル
import-failed-read = ファイルを読み取れませんでした: { $detail }
import-failed-format = { $file } は Archeion のプロファイルではありません。
import-failed-newer = { $file } は新しいバージョンの Archeion で作成されています。インポートするには Archeion を更新してください。
export-failed = プロファイルを保存できませんでした: { $detail }
import-backup = バックアップからプロファイルをインポート…
import-backup-hint = Archeion のバックアップフォルダーから、各バックアップがそこに保存しているコピーを使ってプロファイルを追加します。たとえば新しいコンピューターで使います。バックアップはそのままの場所に残ります。
import-backup-busy = バックアップからプロファイルをインポートしています。
import-backup-password-title = 暗号化されたバックアップ
import-backup-password-body = このフォルダーのバックアップは暗号化されています。プロファイルを読み取るにはパスワードを入力してください。パスワードは、ほかの暗号化されたプロファイルと同じようにプロファイルと一緒に保存されます。
import-backup-import = インポート
import-backup-none = { $folder } には Archeion のバックアップがありません。archeion.json があるフォルダーを選んでください。
import-backup-no-copy = { $folder } のバックアップは以前のバージョンの Archeion で作成されたため、プロファイルのコピーがありません。これらを使うには、このフォルダーを保存先にした新しいプロファイルを作成し、「これらのバックアップをこのプロファイルで使う」を選んでください。
import-backup-damaged = { $folder } のプロファイルのコピーを読み取れません: { $detail }
import-backup-failed = { $folder } を読み取れませんでした: { $detail }
import-backup-password-not-saved = プロファイルはインポートされましたが、パスワードを保存できませんでした。プロファイルの設定で入力してください。{ $detail }
doc-profiles-files = [ファイル] > [プロファイルをエクスポート…] は、選択したプロファイルのすべてのルール（フォルダー、除外、保存先、形式、スケジュール、統合、チェック、通知）を .archeion-profile ファイルに保存し、保管したり別のコンピューターへ移したりできます。[ファイル] > [プロファイルをインポート…] で再び追加できます。パスワードがファイルに含まれることはなく、暗号化されたプロファイルはインポート後にパスワードを求めます。別のコンピューターでは、インポートしたプロファイルが保存先のバックアップを見つけます。同じコンピューターに 2 回インポートすると、名前に「(2)」が付いたコピーになります。
doc-profiles-from-backup = 各バックアップは、プロファイルのコピーも保存先フォルダーに保存します (archeion.profile.toml、暗号化されている場合は保存先の鍵で封印された archeion.profile.zip)。新しいコンピューターや設定が失われたときは、ファイル > バックアップからプロファイルをインポート… で、選んだフォルダーからコピーを読み取ります。プロファイルはすべてのルールとともに戻り、保存先はそのフォルダーになり (ドライブ文字が変わっていても)、スケジュールはあなたが有効にするまで待機します。暗号化されたバックアップでは、先にパスワードを尋ねます。コピーが単独で使われることはなく、このコマンドだけが読み取ります。
