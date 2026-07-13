from __future__ import annotations

import os
from concurrent.futures import ThreadPoolExecutor

from PIL import Image, ImageDraw

from ...config import log
from ...constants import COMBO_MAP, RANK_MAP, SYNC_MAP
from ...resources import FOTNEWRODIN, SIYUAN, TBFONT, pic_dir
from ..merge.models import PlayedResult, Theme
from ..service import mai
from ..utils.calc import dx_score
from .assets import AssetsImage
from .tools import DrawText, song_chart

_DRAW_WORKERS = max(2, min(8, (os.cpu_count() or 4)))


def get_char_width(o: int) -> int:
    widths = [
        (126, 1),
        (159, 0),
        (687, 1),
        (710, 0),
        (711, 1),
        (727, 0),
        (733, 1),
        (879, 0),
        (1154, 1),
        (1161, 0),
        (4347, 1),
        (4447, 2),
        (7467, 1),
        (7521, 0),
        (8369, 1),
        (8426, 0),
        (9000, 1),
        (9002, 2),
        (11021, 1),
        (12350, 2),
        (12351, 1),
        (12438, 2),
        (12442, 0),
        (19893, 2),
        (19967, 1),
        (55203, 2),
        (63743, 1),
        (64106, 2),
        (65039, 1),
        (65059, 0),
        (65131, 2),
        (65279, 1),
        (65376, 2),
        (65500, 1),
        (65510, 2),
        (120831, 1),
        (262141, 2),
        (1114109, 1),
    ]
    if o == 0xE or o == 0xF:
        return 0
    for num, wid in widths:
        if o <= num:
            return wid
    return 1


def coloum_width(s: str) -> int:
    res = 0
    for ch in s:
        res += get_char_width(ord(ch))
    return res


def change_column_width(s: str, len: int) -> str:
    res = 0
    slist = []
    for ch in s:
        res += get_char_width(ord(ch))
        if res <= len:
            slist.append(ch)
    return "".join(slist)


class ScoreBaseImage(AssetsImage):
    theme = Theme.CIRCLE

    def __init__(
        self, image: Image.Image = None, theme: Theme = Theme.CIRCLE
    ) -> None:
        super().__init__()
        self._im = image
        self.theme = theme
        dr = ImageDraw.Draw(self._im)
        self._sy = DrawText(dr, SIYUAN)
        self._tb = DrawText(dr, TBFONT)
        self._fot = DrawText(dr, FOTNEWRODIN)

        self._title_bg = self._themed_image(theme, "title.png")
        self._title_lengthen_bg = self._themed_image(theme, "title_lengthen.png")

    def whiledraw(self, data: list[PlayedResult], dx: bool = False, list_y: int = 0):
        """Draw score cards. Cards are rendered in a thread pool then composited."""
        self.whiledraw_sections([(data, dx, list_y)])

    def whiledraw_sections(
        self,
        sections: list[tuple[list[PlayedResult], bool, int]],
    ) -> None:
        """Draw multiple score grids (e.g. B35 + B15) with one parallel batch."""
        gap = 114
        dx_step = 276
        start_x = 16

        # (card_job, paste_xy)
        prepared: list[
            tuple[
                tuple[PlayedResult, float, int, Image.Image, Image.Image | None],
                tuple[int, int],
            ]
        ] = []

        for data, dx, list_y in sections:
            if not data:
                continue
            if list_y == 0:
                initial_y = 1085 if dx else 235
            else:
                initial_y = list_y

            # Resolve music metadata on the main thread (shared song tables).
            for num, info in enumerate(data):
                row, col = divmod(num, 5)
                x = start_x + col * dx_step
                y = initial_y + row * gap
                li = int(info.level_index)
                song = mai.total_list.by_id(info.song_id)
                if song is None:
                    log.warning(
                        f"曲库缺少曲目 {info.song_id}（{info.song_name}），跳过绘制"
                    )
                    continue
                dxscore = mai.dx_score_of(info.song_id, li) or 0
                ds = mai.resolve_level_value(info.song_id, li)
                if ds is None:
                    ds = float(info.level_value or 0)
                    log.warning(
                        f"曲库缺少谱面 {info.song_id}-{li}，使用成绩自带定数 {ds}"
                    )
                dx_star_im = None
                if dxscore and (star := dx_score(info.dx_score / dxscore * 100)) != 0:
                    dx_star_im = self._dx_star_bg[star - 1]
                prepared.append(
                    ((info, ds, dxscore, self._diff_bg[li], dx_star_im), (x, y))
                )

        if not prepared:
            return

        theme = self.theme
        id_colors = self._id_text_color
        diff_colors = self._diff_text_color

        def _render(
            job: tuple[PlayedResult, float, int, Image.Image, Image.Image | None],
        ) -> Image.Image:
            info, ds, dxscore, diff_bg, dx_star_im = job
            return _render_score_card(
                info,
                ds=ds,
                dxscore=dxscore,
                theme=theme,
                diff_bg=diff_bg,
                dx_star_im=dx_star_im,
                id_text_color=id_colors[int(info.level_index)],
                diff_text_color=diff_colors[int(info.level_index)],
            )

        jobs = [item[0] for item in prepared]
        if len(jobs) == 1:
            tiles = [_render(jobs[0])]
        else:
            workers = min(_DRAW_WORKERS, len(jobs))
            with ThreadPoolExecutor(max_workers=workers) as pool:
                tiles = list(pool.map(_render, jobs))

        for tile, (_job, (x, y)) in zip(tiles, prepared):
            self._im.alpha_composite(tile, (x, y))


def _rate_icon_name(rate) -> str:
    """Map PlayedResult.rate to the UI rank icon stem."""
    if rate is None:
        return "D"
    text = str(rate)
    # str Enum may stringify as "RateType.SSSP" in some cases; prefer .value.
    value = getattr(rate, "value", text)
    if isinstance(value, str) and value.islower():
        return RANK_MAP.get(value, value.upper())
    return str(value)


def _render_score_card(
    info: PlayedResult,
    *,
    ds: float,
    dxscore: int,
    theme: Theme,
    diff_bg: Image.Image,
    dx_star_im: Image.Image | None,
    id_text_color: tuple[int, int, int, int],
    diff_text_color: tuple[int, int, int, int],
) -> Image.Image:
    """Render one b50-style score card. Safe to call from worker threads."""
    card = diff_bg.copy()
    draw = ImageDraw.Draw(card)
    sy = DrawText(draw, SIYUAN)
    tb = DrawText(draw, TBFONT)

    cover = Image.open(song_chart(info.song_id)).convert("RGBA").resize((75, 75))
    type_ = Image.open(pic_dir / f"{info.type.upper()}.png").convert("RGBA").resize(
        (37, 14)
    )
    rate = Image.open(
        pic_dir / theme.value / f"UI_TTR_Rank_{_rate_icon_name(info.rate)}.png"
    ).convert("RGBA").resize((63, 28))

    card.alpha_composite(cover, (12, 12))
    card.alpha_composite(type_, (51, 91))
    card.alpha_composite(rate, (92, 78))

    if info.fc:
        fc = Image.open(
            pic_dir / f"UI_MSS_MBase_Icon_{COMBO_MAP[info.fc]}.png"
        ).convert("RGBA").resize((34, 34))
        card.alpha_composite(fc, (154, 77))
    if info.fs:
        fs = Image.open(
            pic_dir / f"UI_MSS_MBase_Icon_{SYNC_MAP[info.fs]}.png"
        ).convert("RGBA").resize((34, 34))
        card.alpha_composite(fs, (185, 77))

    if dx_star_im is not None:
        card.alpha_composite(dx_star_im.resize((47, 26)), (217, 80))

    tb.draw(26, 98, 13, info.song_id, id_text_color, anchor="mm")

    title = info.song_name
    if coloum_width(title) > 18:
        title = change_column_width(title, 17) + "..."
    sy.draw(93, 14, 14, title, diff_text_color, anchor="lm")
    tb.draw(93, 38, 30, f"{info.achievements:.4f}%", diff_text_color, anchor="lm")
    tb.draw(219, 65, 15, f"{info.dx_score}/{dxscore}", diff_text_color, anchor="mm")
    tb.draw(93, 65, 15, f"{ds} -> {info.rating}", diff_text_color, anchor="lm")
    return card
