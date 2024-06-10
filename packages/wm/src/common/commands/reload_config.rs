use anyhow::Context;
use tracing::{info, warn};

use crate::{
  containers::traits::{CommonGetters, TilingSizeGetters},
  user_config::{ParsedConfig, UserConfig},
  windows::traits::WindowGetters,
  wm_event::WmEvent,
  wm_state::WmState,
};

pub fn reload_config(
  state: &mut WmState,
  config: &mut UserConfig,
) -> anyhow::Result<()> {
  info!("Config reloaded.");

  // Keep reference to old config for comparison.
  let old_config = config.value.clone();

  // Re-evaluate user config file and set its values in state.
  tokio::task::block_in_place(|| {
    let rt = tokio::runtime::Handle::current();
    rt.block_on(config.reload())
  })?;

  // TODO: Run window rules on all windows.

  update_workspace_configs(state, config)?;

  update_container_gaps(state, config);

  update_window_effects(&old_config, state, config)?;

  // Clear active binding modes.
  state.binding_modes = Vec::new();

  // Redraw full container tree.
  let root_container = state.root_container.clone();
  state
    .pending_sync
    .containers_to_redraw
    .push(root_container.into());

  // Emit the updated config.
  state.emit_event(WmEvent::UserConfigChanged {
    config_path: config
      .path
      .to_str()
      .context("Invalid config path.")?
      .to_string(),
    config_string: config.value_str.clone(),
    parsed_config: config.value.clone(),
  });

  Ok(())
}

/// Update configs of active workspaces.
fn update_workspace_configs(
  state: &mut WmState,
  config: &UserConfig,
) -> anyhow::Result<()> {
  let workspaces = state.workspaces();

  for workspace in &workspaces {
    let workspace_config = config
      .value
      .workspaces
      .iter()
      .find(|config| config.name == workspace.config().name);

    match workspace_config {
      Some(workspace_config) => {
        workspace.set_config(workspace_config.clone());
      }
      // When the workspace config is not found, the current name of the
      // workspace has been removed. So, we reassign the first suitable
      // workspace config to the workspace.
      None => {
        let monitor = workspace.monitor().context("No monitor.")?;
        let inactive_config =
          config.workspace_config_for_monitor(&monitor, &workspaces);

        if let Some(inactive_config) = inactive_config {
          workspace.set_config(inactive_config.clone());
        } else {
          warn!(
            "Unable to update workspace config. No available workspace configs."
          );
        }
      }
    }
  }

  Ok(())
}

/// Updates outer gap of workspaces and inner gaps of tiling containers.
fn update_container_gaps(state: &mut WmState, config: &UserConfig) {
  let tiling_containers = state
    .root_container
    .self_and_descendants()
    .into_iter()
    .filter_map(|container| container.as_tiling_container().ok());

  for container in tiling_containers {
    container.set_inner_gap(config.value.gaps.inner_gap.clone());
  }

  for workspace in state.workspaces() {
    workspace.set_outer_gap(config.value.gaps.outer_gap.clone());
  }
}

fn update_window_effects(
  old_config: &ParsedConfig,
  state: &mut WmState,
  config: &UserConfig,
) -> anyhow::Result<()> {
  let focused_container =
    state.focused_container().context("No focused container.")?;

  let window_effects = &config.value.window_effects;
  let old_window_effects = &old_config.window_effects;

  // Window border effects are left as is if set to `null` in the config.
  // However, when transitioning from a non-null value to `null`, we have
  // to reset to the system default.
  if window_effects.focused_window.border_color.is_none()
    && old_window_effects.focused_window.border_color.is_some()
  {
    if let Ok(window) = focused_container.as_window_container() {
      _ = window.native().set_border_color(None);
    }
  }

  if window_effects.other_windows.border_color.is_none()
    && old_window_effects.other_windows.border_color.is_some()
  {
    let unfocused_windows = state
      .windows()
      .into_iter()
      .filter(|window| window.id() != focused_container.id());

    for window in unfocused_windows {
      _ = window.native().set_border_color(None);
    }
  }

  state.pending_sync.reset_window_effects = true;

  Ok(())
}