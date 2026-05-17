// Shared "is a key-capture surface focused" flag. Single source of truth used
// both by the popup / step-editor capture code (to suppress engine synth) and
// by the keyboard-mirror feature (to suppress the flash animation while the
// user is typing into a capture box).
//
// Callers MUST go through setCaptureActive(active) rather than calling
// `invoke('set_capture_active', ...)` directly — the helper keeps the local
// flag in sync with the engine and the IPC call.

import { invoke } from './ipc.js';

let active = false;

export function isCaptureActive() {
  return active;
}

export function setCaptureActive(next) {
  active = !!next;
  // Fire-and-forget — IPC failure is non-fatal (the engine just keeps
  // synthesising); the local flag is what gates the keyboard mirror.
  invoke('set_capture_active', { active }).catch(() => {});
}
