# Shims for running copied Tk Environment dialog code without its full GUI
# startup.  Sourced by harness.py after new-server.tcl and before any dialog
# file.  Nothing here modifies actr7.x.

set tcl_env_dir [pwd]
set top_dir [pwd]
set text_font "TkFixedFont"
set list_font "TkFixedFont"
set button_font "TkDefaultFont"
set label_font "TkDefaultFont"
set intro_s_font "TkDefaultFont"
set global_status ""
set local_connection 1
set up_and_running 0
set current_open_model ""
set current_file_window ""
set currently_open_files ""
set global_current_model_name "No Current Model"
set during_reload 0
set run_button_time "10.0"
set ::tk_message_boxes ""
set ::tk_open_file ""
set ::tk_save_file ""
set ::shim_counter 0
set ::shim_update_scripts {}

array set options_array {
  use_localhost 0
  use_smart_load 0
  save_before_reload 0
  save_backups 0
  kill_model_windows 0
  edit_warn 0
  update_when_stepped 1
  use_env_window 1
}

# The dialogs place their buttons into this frame hierarchy.
proc control_panel_name {} { return ".control_panel.frame.c.contents" }
catch {destroy .control_panel}
frame .control_panel
frame .control_panel.frame
canvas .control_panel.frame.c
frame .control_panel.frame.c.contents

proc new_variable_name {base} {
  incr ::shim_counter
  return "${base}_$::shim_counter"
}

proc get_configuration {args} { return "520x360+20+20" }

proc update_text_pane {w text} {
  $w configure -state normal
  $w delete 1.0 end
  $w insert 1.0 $text
  $w configure -state disabled
}

proc set_update_script {win script} {
  lappend ::shim_update_scripts $script
}

proc run_shim_updates {} {
  foreach script $::shim_update_scripts {
    uplevel #0 $script
  }
}

proc record_new_window {args} {}
proc record_window_with_model {args} {}
proc kill_all_model_windows {args} {}
proc save_model {args} {}
proc close_model {args} {}
proc append_data {args} {}
proc console {args} {}
proc set_return_result {args} {}
proc clean_exit {} {}

proc report_status {s} {
  set ::global_status $s
  append ::tk_message_boxes "report_status: $s\n"
}

proc tk_messageBox {args} {
  append ::tk_message_boxes "[join $args { }]\n"
  if {[info exists ::shim_messagebox_answer]} {
    return $::shim_messagebox_answer
  }
  return ok
}

proc tk_getOpenFile {args} { return $::tk_open_file }
proc tk_getSaveFile {args} { return $::tk_save_file }

proc currently_selected_model {} {
  return $::global_current_model_name
}
