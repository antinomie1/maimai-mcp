from pathlib import Path

from .config import maiconfig

if maiconfig.maimaidx_path:
    static = Path(maiconfig.maimaidx_path)
else:
    raise ValueError(
        "未检测到静态文件夹 `static`，请在 .env 中配置 MAIMAIDX_PATH 为 static 的绝对路径"
    )


# 静态资源路径
font_dir = static / "font"
data_dir = static / "data"
mai_dir = static / "mai"
pic_dir = mai_dir / "pic"
cover_dir = mai_dir / "cover"
plate_dir = mai_dir / "plate"
shougou_dir = mai_dir / "shougou"
plate_version_dir = mai_dir / "plate_version"
plate_table_dir = mai_dir / "plate_table"
rating_table_dir = mai_dir / "rating_table"

data_dir.mkdir(parents=True, exist_ok=True)
plate_table_dir.mkdir(parents=True, exist_ok=True)
rating_table_dir.mkdir(parents=True, exist_ok=True)

# 路径文件
alias_file = data_dir / "music_alias.json"
lxns_alias_file = data_dir / "lxns_music_alias.json"
local_alias_file = data_dir / "local_music_alias.json"
music_file = data_dir / "music_data.json"
lxns_music_file = data_dir / "lxns_music_data.json"
chart_file = data_dir / "music_chart.json"
plate_file = data_dir / "plate_data.json"
merge_music_file = data_dir / "merge_music_data.json"
merge_alias_file = data_dir / "merge_music_alias.json"

# 字体路径
SIYUAN = font_dir / "ResourceHanRoundedCN-Bold.ttf"
SHANGGUMONO = font_dir / "ShangguMonoSC-Regular.otf"
TBFONT = font_dir / "Torus SemiBold.otf"
FOTNEWRODIN = font_dir / "FOT-NewRodin Pro EB.otf"
