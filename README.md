# dualsense-mac-mapper

#### Test environment
- Macbook M2 (macOS 15.5)
- Python 3.13.5
  

# 🎮 DualSense Mac Mapper

A Python-based key remapper for macOS (Apple Silicon) that maps every button and axis on a PS5 DualSense controller to keyboard keys — including macros, deadzone filtering, and customizable mappings.

---

## 🚀 Features

- Full button remapping (buttons 0–14)
- Analog stick and trigger normalization (added as virtual keys 15–24)
- Deadzone support and trigger threshold control
- Custom macro definition system with randomized delays
- Pygame + pynput based — no special kernel extensions needed

---

## 🔧 Required libraries
```bash
pip install pygame pynput
```