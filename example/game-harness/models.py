from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class FlagItem:
    flag_id: Optional[int]
    flag_value: str
    raw: dict
