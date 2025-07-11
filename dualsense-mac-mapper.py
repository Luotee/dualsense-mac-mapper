import random
import pygame
import threading
import time
import signal
import atexit
import sys
from pynput.keyboard import Controller, Key
 
# === Setup ===
keyboard = Controller()
pygame.init()
pygame.joystick.init()

AXIS_DEADZONE = 0.4
TRIGGER_THRESHOLD = 0.5

key_states = {}
lock = threading.Lock()
macro_triggered = set()

# === Macro definition ===
def macro_A():
    return [
        (Key.left, 'down', 0.05, 0.08),
        (Key.left, 'up', 0.015, 0.025),
        (Key.right, 'down', 0.05, 0.06),
        (Key.right, 'up', 0.015, 0.025),
    ]

# === Key Mapping ===
key_mapping = {
    0: 'x',            # Cross（叉叉）
    1: '3',            # Circle（圈圈）
    2: 'z',            # Square（正方形）
    3: '',             # Triangle（三角）
    4: '',             # Share（拍照鍵）
    5: '',             # PS 按鈕
    6: '',             # Options（Menu 鍵）
    7: '',             # L3
    8: '',             # R3
    9: '1',            # L1
    10: Key.shift,     # R1
    11: Key.up,        # D-pad ↑
    12: Key.down,      # D-pad ↓
    13: Key.left,      # D-pad ←
    14: Key.right,     # D-pad →
    15: Key.up,        # L-stick ↑
    16: Key.down,      # L-stick ↓
    17: Key.left,      # L-stick ←
    18: Key.right,     # L-stick →
    19: '',            # R-stick ↑
    20: '',            # R-stick ↓
    21: '',            # R-stick ←
    22: '',            # R-stick →
    23: macro_A,       # L2（類比板機）觸發巨集
    24: 'a',           # R2（類比板機）
}

# === Reverse Mapping for key release tracking ===
reverse_mapping = {}
for idx, val in key_mapping.items():
    if callable(val):  # Skip macros
        continue
    if val in reverse_mapping:
        reverse_mapping[val].append(idx)
    else:
        reverse_mapping[val] = [idx]

# === Utility functions ===
def key_press(key):
    if key != '':
        try:
            keyboard.press(key)
        except ValueError:
            print(f"[Invalid press] {key}")

def key_release(key):
    if key != '':
        try:
            keyboard.release(key)
        except ValueError:
            print(f"[Invalid release] {key}")
    # Reset key_states for all indexes mapped to this key
    for idx in reverse_mapping.get(key, []):
        with lock:
            key_states[idx] = False

def press_loop(index, key):
    with lock:
        if key_states.get(index):
            return
        key_states[index] = True
    try:
        if callable(key):
            macro_triggered.add(index)
            while True:
                with lock:
                    if index not in macro_triggered:
                        break
                for k, act, tmin, tmax in key():
                    with lock:
                        if index not in macro_triggered:
                            break
                    if k and act == 'down': key_press(k)
                    if k and act == 'up': key_release(k)
                    # sleep時細分檢查flag，確保macro可即時中斷
                    total_sleep = random.uniform(tmin, tmax)
                    slept = 0
                    while slept < total_sleep:
                        with lock:
                            if index not in macro_triggered:
                                break
                        s = min(0.01, total_sleep - slept)
                        time.sleep(s)
                        slept += s
                    with lock:
                        if index not in macro_triggered:
                            break
        else:
            key_press(key)
            while True:
                with lock:
                    if not key_states.get(index):
                        break
                time.sleep(0.01)
    finally:
        if not callable(key):
            key_release(key)
        with lock:
            key_states[index] = False
            macro_triggered.discard(index)

def start_key(index):
    key = key_mapping.get(index)
    if key == '' or key is None:
        return
    with lock:
        if key_states.get(index):
            return
    t = threading.Thread(target=press_loop, args=(index, key), daemon=True)
    t.start()


def stop_key(index):
    with lock:
        key_states[index] = False
        macro_triggered.discard(index)

def process_joystick(joystick):
    lx = joystick.get_axis(0)
    ly = joystick.get_axis(1)
    rx = joystick.get_axis(2)
    ry = joystick.get_axis(3)
    l2 = joystick.get_axis(4)
    r2 = joystick.get_axis(5)

    # 左類比蘑菇頭
    start_key(15) if ly < -AXIS_DEADZONE else stop_key(15)
    start_key(16) if ly > AXIS_DEADZONE else stop_key(16)
    start_key(17) if lx < -AXIS_DEADZONE else stop_key(17)
    start_key(18) if lx > AXIS_DEADZONE else stop_key(18)

    # 右類比蘑菇頭
    start_key(19) if ry < -AXIS_DEADZONE else stop_key(19)
    start_key(20) if ry > AXIS_DEADZONE else stop_key(20)
    start_key(21) if rx < -AXIS_DEADZONE else stop_key(21)
    start_key(22) if rx > AXIS_DEADZONE else stop_key(22)

    # 板機
    start_key(23) if l2 > -1 + TRIGGER_THRESHOLD else stop_key(23)
    start_key(24) if r2 > -1 + TRIGGER_THRESHOLD else stop_key(24)

# === Cleanup ===
def release_all_keys():
    for index, active in key_states.items():
        if active:
            stop_key(index)
            k = key_mapping.get(index)
            if k and not callable(k):
                key_release(k)
    try:
        pygame.joystick.quit()
        pygame.quit()
    except Exception:
        pass

atexit.register(release_all_keys)
signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))

def main():
    joystick = None
    last_joystick_name = None
    had_joystick = False
    while True:
        try:
            # 每秒檢查搖桿數量
            if pygame.joystick.get_count() == 0:
                if had_joystick:
                    print("⚠️ 搖桿已斷線，請重新連接...")
                    had_joystick = False
                joystick = None
                time.sleep(1)
                continue
            if joystick is None or not joystick.get_init():
                pygame.joystick.quit()
                pygame.joystick.init()
                joystick = pygame.joystick.Joystick(0)
                joystick.init()
                last_joystick_name = joystick.get_name()
                print(f"🎮 已啟用: {last_joystick_name}")
                had_joystick = True
            # 處理搖桿事件
            pygame.event.pump()
            process_joystick(joystick)
            for i in range(15):
                if joystick.get_button(i):
                    start_key(i)
                else:
                    stop_key(i)
            time.sleep(0.01)
        except KeyboardInterrupt:
            break
        except Exception as e:
            print(f"[錯誤] {e}")
            joystick = None
            time.sleep(1)
    release_all_keys()

if __name__ == "__main__":
    main()