use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{
  mpsc::{self, UnboundedReceiver},
  Mutex,
};

use crate::user_config::{
  BindingModeConfig, KeybindingConfig, UserConfig,
};

use super::{EventWindow, NativeWindow};

#[derive(Debug)]
pub enum PlatformEvent {
  DisplaySettingsChanged,
  KeybindingTriggered(KeybindingConfig),
  MouseMove,
  WindowDestroyed(NativeWindow),
  WindowFocused(NativeWindow),
  WindowHidden(NativeWindow),
  WindowLocationChanged(NativeWindow),
  WindowMinimized(NativeWindow),
  WindowMinimizeEnded(NativeWindow),
  WindowMovedOrResized(NativeWindow),
  WindowShown(NativeWindow),
  WindowTitleChanged(NativeWindow),
}

pub struct EventListener {
  pub event_rx: UnboundedReceiver<PlatformEvent>,
  event_window: EventWindow,
}

impl EventListener {
  /// Initializes listener for platform events.
  ///
  /// Creates an instance of `EventListener`.
  pub async fn start(config: &Arc<Mutex<UserConfig>>) -> Result<Self> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let config = config.lock().await;

    let event_window = EventWindow::new(
      event_tx,
      config.value.keybindings.clone(),
      config.value.general.focus_follows_cursor,
    );

    Ok(Self {
      event_rx,
      event_window,
    })
  }

  /// Updates the event listener with the latest user config and the
  /// currently active binding modes.
  pub fn update(
    &mut self,
    config: &UserConfig,
    binding_modes: &Vec<BindingModeConfig>,
  ) {
    // TODO: Modify keybindings based on active binding modes.
    self
      .event_window
      .update_keybindings(config.value.keybindings.clone());

    self
      .event_window
      .enable_mouse_listener(config.value.general.focus_follows_cursor);
  }
}
