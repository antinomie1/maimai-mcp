from enum import Enum


class ServiceName(str, Enum):
    DIVINGFISH = "Diving-Fish"
    LXNS = "Lxns-Network"

    @classmethod
    def get_by_index(cls, index_str: str) -> "ServiceName | None":
        """Accept index (0/1), enum name, value, or short alias (水鱼/落雪/lxns)."""
        key = index_str.strip()
        mapping = {str(i): item for i, item in enumerate(cls)}
        if key in mapping:
            return mapping[key]
        aliases = {
            "0": cls.DIVINGFISH,
            "divingfish": cls.DIVINGFISH,
            "diving-fish": cls.DIVINGFISH,
            "df": cls.DIVINGFISH,
            "水鱼": cls.DIVINGFISH,
            "1": cls.LXNS,
            "lxns": cls.LXNS,
            "lx": cls.LXNS,
            "落雪": cls.LXNS,
        }
        low = key.lower()
        if low in aliases:
            return aliases[low]
        for item in cls:
            if key == item.name or key == item.value or low == item.value.lower():
                return item
        return None

    @classmethod
    def get_help(cls) -> str:
        return "\n".join(
            [f"「{i}」/「{item.name.lower()}」：{item.value}" for i, item in enumerate(cls)]
        )


class Category(str, Enum):
    DEFAULT = "default"
    COMPLETED = "completed"
    UNFINISHED = "unfinished"
    NOTPLAYED = "notplayed"


class Theme(str, Enum):
    """UI theme. Default for new users is CIRCLE; use PRISM_PLUS only when chosen."""

    CIRCLE = "circle"
    PRISM_PLUS = "prism_plus"

    @classmethod
    def get_by_index(cls, index_str: str) -> "Theme | None":
        """Accept index (0=circle, 1=prism_plus) or theme name."""
        key = index_str.strip()
        mapping = {str(i): item for i, item in enumerate(cls)}
        if key in mapping:
            return mapping[key]
        low = key.lower().replace("-", "_").replace(" ", "_")
        aliases = {
            "0": cls.CIRCLE,
            "circle": cls.CIRCLE,
            "1": cls.PRISM_PLUS,
            "prism": cls.PRISM_PLUS,
            "prism_plus": cls.PRISM_PLUS,
            "prismplus": cls.PRISM_PLUS,
        }
        if low in aliases:
            return aliases[low]
        for item in cls:
            if key == item.name or low == item.value.lower():
                return item
        return None

    @classmethod
    def get_help(cls) -> str:
        return (
            "默认 circle。\n"
            + "\n".join(
                [f"「{i}」/「{item.value}」：{item.value}" for i, item in enumerate(cls)]
            )
        )
