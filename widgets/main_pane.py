#!/usr/bin/env python3
"""Main dashboard pane — clock, stats, connections."""

import os
import time
from collections import deque
from datetime import datetime

from rich.console import Console
from rich.live import Live
from rich.text import Text

from common import (
    ICON,
    battery_icon,
    brightness_icon,
    get_battery,
    get_bluetooth,
    get_brightness,
    get_cpu_percent,
    get_ram,
    get_volume,
    get_wifi,
    progress_bar,
    render_big_time,
    sparkline,
    volume_icon,
    wifi_icon,
)

WIDTH = 36
PAD = 15  # pad label+value so bars align (widest: "ram  10.5/16G" = 13)
console = Console(width=WIDTH, highlight=False)
cpu_history: deque[float] = deque(maxlen=10)
ram_history: deque[float] = deque(maxlen=10)


def center(line: str) -> str:
    pad = max((WIDTH - len(line)) // 2, 0)
    return " " * pad + line


def load_color(pct: float) -> str:
    """Green < 50%, yellow < 80%, red >= 80%."""
    if pct < 50:
        return "green"
    if pct < 80:
        return "yellow"
    return "red"


def bat_color(pct: int) -> str:
    if pct > 50:
        return "green"
    if pct > 20:
        return "yellow"
    return "red"


def render_clock(text: Text, now: datetime) -> None:
    r1, r2 = render_big_time(
        now.strftime("%H:%M"),
        colon_on=now.second % 2 == 0,
    )
    text.append(center(r1) + "\n", style="bold")
    text.append(center(r2) + "\n", style="bold")
    text.append("\n")
    date_line = now.strftime("%A · %B %d").lower()
    text.append(center(date_line) + "\n", style="dim")


def render_stats(text: Text) -> None:
    cpu = get_cpu_percent()
    cpu_history.append(cpu)
    ram_used, ram_total = get_ram()
    ram_pct = ram_used / ram_total * 100 if ram_total > 0 else 0
    ram_history.append(ram_pct)

    # CPU — orange icon, load-colored sparkline
    text.append(f"   {ICON['cpu']}  ", style="cyan")
    info = f"cpu  {cpu:3.0f}%"
    text.append(f"{info:<{PAD}}")
    text.append(f"{sparkline(cpu_history)}\n", style=load_color(cpu))

    # RAM — magenta icon, load-colored sparkline
    text.append(f"   {ICON['ram']}  ", style="magenta")
    info = f"ram  {ram_used:.1f}/{ram_total:.0f}G"
    text.append(f"{info:<{PAD}}")
    text.append(f"{sparkline(ram_history)}\n", style=load_color(ram_pct))

    _render_battery(text)
    _render_volume(text)
    _render_brightness(text)


def _render_battery(text: Text) -> None:
    bat = get_battery()
    if not bat:
        return
    pct, status, _time_rem = bat
    icon = battery_icon(pct, status)
    color = bat_color(pct)
    text.append(f"   {icon}  ", style=color)
    info = f"bat  {pct:3d}%"
    text.append(f"{info:<{PAD}}")
    text.append(f"{progress_bar(pct)}\n", style=color)


def _render_volume(text: Text) -> None:
    vol, muted = get_volume()
    vicon = volume_icon(vol, muted=muted)
    if muted:
        text.append(f"   {vicon}  ", style="dim")
        info = "vol  muted"
        text.append(f"{info:<{PAD}}")
        text.append(f"{progress_bar(vol)}\n", style="dim")
    else:
        text.append(f"   {vicon}  ", style="blue")
        info = f"vol  {vol:3d}%"
        text.append(f"{info:<{PAD}}")
        text.append(f"{progress_bar(vol)}\n", style="blue")


def _render_brightness(text: Text) -> None:
    bri = get_brightness()
    if bri is None:
        return
    bicon = brightness_icon(bri)
    text.append(f"   {bicon}  ", style="yellow")
    info = f"bri  {bri:3d}%"
    text.append(f"{info:<{PAD}}")
    text.append(f"{progress_bar(bri)}\n", style="yellow")


def render_connections(text: Text) -> None:
    wifi = get_wifi()
    if wifi:
        ssid, signal = wifi
        wicon = wifi_icon(signal)
        display_ssid = ssid[:14] if len(ssid) > 14 else ssid
        text.append(f"   {wicon}  ", style="cyan")
        text.append(f"{display_ssid} ({signal}%)\n")
    else:
        text.append(f"   {ICON['wifi_off']}  ", style="dim")
        text.append("offline\n", style="dim")

    bt = get_bluetooth()
    if bt:
        text.append(f"   {bt}\n", style="blue")


def content_lines() -> int:
    """Count how many lines the content occupies (for vertical centering)."""
    # clock: 2 + 1 blank + 1 date = 4, stats: up to 5, connections: up to 2
    # separators: 2 + 2 = 4 blank lines between sections
    return 15


def render() -> Text:
    text = Text()
    try:
        term_h = os.get_terminal_size().lines
    except OSError:
        term_h = 23
    top_pad = max((term_h - content_lines()) // 2, 0)
    text.append("\n" * top_pad)
    render_clock(text, datetime.now())  # noqa: DTZ005
    text.append("\n\n")
    render_stats(text)
    text.append("\n\n")
    render_connections(text)
    return text


console.clear()
with Live(render(), console=console, refresh_per_second=2) as live:
    while True:
        live.update(render())
        time.sleep(1)
