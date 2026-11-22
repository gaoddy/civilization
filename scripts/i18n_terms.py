#!/usr/bin/env python3
"""国际化文本 id 的词表与命名规则。

生成器与客户端共享这些常量：本文件只描述标识，不承载任何中文文案。
"""

import re

CARD_TYPES = ("minion", "effect")
ERAS = ("prehistoric", "primitive", "classical", "feudal", "industrial", "modern", "future")
KEYWORD_BASES = ("battlecry", "deathrattle", "reborn", "taunt", "divine_shield",
                 "windfury", "contagion", "ignore_taunt", "sow", "evolve",
                 "breed", "assemble", "poison", "armor", "stealth")
EVOLVE_CHOICES = ("stats", "divine_shield", "reborn")
STAT_LAYERS = ("initial", "permanent", "turn", "aura", "damage")
GENERATED_PREFIXES = (
    "card.", "skill.",
    "enum.card_type.", "enum.playstyle.", "enum.taxon.", "enum.era.", "enum.keyword.", "enum.subtype.",
)
MANUAL_PREFIXES = ("ui.", "error.", "enum.evolve.", "enum.stat_layer.",
                   "enum.friend_status.", "enum.stat_source_kind.")
# 已废弃的生成前缀：合并时直接丢弃，不再视为未知前缀报错。
LEGACY_PREFIXES = ("enum.tribe.", "token.")

_TRAILING_VALUE = re.compile(r"\s+(?:N|\d+)$")


def keyword_slug(english_name):
    """把关键词英文名转成文本 id 片段：去掉末尾的 N，转小写，空格转下划线。"""
    name = _TRAILING_VALUE.sub("", english_name.strip())
    return name.lower().replace(" ", "_")
