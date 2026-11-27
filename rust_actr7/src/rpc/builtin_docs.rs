//! Documentation strings for the commands the Rust engine implements.
//!
//! Mirrors the `documentation` argument passed to `add-act-r-command` in the
//! Lisp sources. The Tcl Environment retrieves these through the dispatcher's
//! `check` method.

/// Command name and documentation, in `BUILTINS` order.
pub const BUILTIN_DOCS: &[(&str, &str)] = &[
    ("run", "Run the ACT-R scheduler for up to a specified amount of time. Params: time {real-time}."),
    ("run-full-time", "Run the ACT-R scheduler for the specified amount of time. Params: time {real-time}."),
    ("run-until-time", "Run the ACT-R scheduler up to the indicated time. Params: end-time {real-time}."),
    ("run-n-events", "Run the indicated number of events from the ACT-R scheduler. Params: event-count {real-time}."),
    ("step-once", "Run the ACT-R scheduler for a single event. No params."),
    ("reset", "Reset the ACT-R scheduler and all models. No params."),
    ("reload", "Reload the last ACT-R model file which was loaded. Params: {compile}."),
    ("clear-all", "Reset ACT-R to its initial state, deleting all models and clearing the event queue. No params."),
    ("load-act-r-model", "Loads the ACT-R model file indicated. Params: pathname {compile}."),
    ("load-act-r-code", "Loads the Lisp code file indicated. Params: pathname {compile}."),
    ("add-act-r-command", "Register a command with the ACT-R dispatcher. Params: name underlying-function {documentation} {single-instance} {local-name} {encoded}."),
    ("remove-act-r-command", "Remove a command from the ACT-R dispatcher. Params: name."),
    ("define-module", "Create a new module. Params: name (buffer-spec*) (param-spec*) {< version, documentation, creation , reset, query, request, buffer-mod, params, delete, notify-on-clear, update, warning, search, offset, run-start, run-end, required, requires >}"),
    ("undefine-module", "Remove the named module from the system.  Params: module-name."),
    ("start-pipe-connection", "Attempt to switch a connection to using pipes returns true if pipe connection available. Params: id."),
    ("goal-focus", "Schedule a chunk to enter the goal buffer at the current time or print the current goal buffer chunk. Params: {chunk-name}."),
    ("mod-focus", "Modify the chunk in the goal buffer using the slots and values provided. Params: '{<slot-name> <new-slot-value>}'*"),
    ("set-parameter-value", "Sets the current value of a parameter in the current model. Params: parameter-name 'new-value'."),
    ("get-parameter-value", "Return the current value of a parameter in the current model. Params: parameter-name."),
    ("set-system-parameter-value", "Sets the current value of a system parameter. Params: system-parameter-name new-value."),
    ("get-system-parameter-value", "Return the current value of a system parameter. Params: system-parameter-name."),
    ("current-model", "Get the name of the current model if there is one. No params."),
    ("set-current-model", "Select the named model as the current model, or clear the current model when passed nil. Params: model-name."),
    ("mp-models", "Return a list of all existing model names. No params."),
    ("creating-model", "Signal for a model being created for monitoring purposes. Params: model-name."),
    ("deleting-model", "Signal for a model being deleted for monitoring purposes. Params: model-name."),
    ("run-start", "Signal that the system is starting to run for monitoring. Params: run-start-time."),
    ("run-stop", "Signal that the system has stopped running for monitoring. Params: run-end-time."),
    ("reset-start", "Signal that a reset has begun for monitoring. No params."),
    ("reset-step1", "Signal for the primary model reset point (before default parameters are set, modules primary resets, and code evaluated) for monitoring purposes.  No params."),
    ("clear-all-start", "Signal that a clear-all has begun for monitoring. No params."),
    ("start-stepper", "Start the stepper control interface if possible. Returns t and an id or nil and a failure string.  Params: step-all? tutor-mode? next-event-cmd update-cmd"),
    ("stop-stepper", "Stop the stepper control interface.  Params: stepper-id."),
    ("step-stepper", "Advance the stepper control interface when it is waiting. Returns: success error-msg. Params: stepper-id {stop? {step-type step-value}}"),
    ("update-stepper", "Stepper interface command for getting the detailed information. Returns: strings for the three information panels for the given item. Params: seleted-item"),
    ("set-stepper-ready", "Stepper interface command for signaling that code which opened a stepper is ready to start stepping. Params: stepper-id."),
    ("set-stepper-tutoring", "Stepper interface command for changing the tutor mode. Params: stepper-id tutoring-value."),
    ("set-stepper-step-all", "Stepper interface command for changing the step-all mode. Params: stepper-id step-all-value."),
    ("new-symbol", "Create new names outside of a model. Params: {name-prefix}"),
    ("mp-time", "Return the current simulation time in seconds. No params."),
    ("mp-time-ms", "Return the current simulation time in milliseconds. No params."),
    ("model-output", "Send a string to the ACT-R model-trace. Params: model-output-string."),
    ("command-output", "Send a string to the ACT-R command-trace. Params: command-output-string."),
    ("print-warning", "Send a string to the ACT-R warning-trace. Params: warning-string."),
    ("act-r-output", "Send a string to the ACT-R general-trace. Params: output-string."),
    ("buffer-chunk", "Print the contents of specified buffers. Params: {buffer-name}*."),
    ("buffer-status", "Print the status information for a buffer and its module. Params: {buffer-name}*."),
    ("buffer-read", "Return the name of the chunk in a buffer. Params: buffer-name."),
    ("buffer-slot-value", "Return the value of a slot for the chunk in a buffer. Params: buffer-name slot-name."),
    ("clear-buffer", "Clear the chunk from a buffer. Params: buffer-name."),
    ("set-buffer-chunk", "Copy a chunk into a buffer or create a chunk given a chunk-spec. Params: buffer-name chunk-name-or-spec {requested?}"),
    ("mod-chunk", "Modify the contents of the specified chunk with the list of slots and values provided. Params: chunk-name '{slot-name new-slot-value}'*"),
    ("mod-buffer-chunk", "Modify a chunk in a buffer. Params: buffer-name 'modifications'."),
    ("extend-possible-slots", "Add a new slot name which can be used for any chunk, and if warn is true (the default) then print a warning if the slot specified already exists. Params: slot-name {warn}"),
    ("schedule-event-relative", "Create an event to occur at the specified amount of time from now. Params: time-delay action { < params, module, destination, priority, maintenance, output, time-in-ms, details, precondition > }."),
    ("schedule-event-now", "Create an event to occur at the current time. Params: time-delay action { < params,  module, destination, priority, maintenance, output, details, precondition > }."),
    ("schedule-event", "Create an event to occur at the specified time. Params: time action { < params, module, destination, priority, maintenance, output, time-in-ms, details, precondition > }."),
    ("schedule-break", "Create a break event to force the current run to stop at the time indicated. Params: time {< details, priority, time-in-ms > }"),
    ("schedule-break-relative", "Create a break event to force the current run to stop after the specified amount of time. Params: time-delay {< details, priority, time-in-ms > }"),
    ("schedule-set-buffer-chunk", "Create an event to occur at the specified amount of time from now to place a chunk in a buffer. Params: buffer-name chunk-name-or-spec time-delay { <  module,  priority,  output, time-in-ms, requested > }."),
    ("schedule-mod-buffer-chunk", "Create an event to occur at the specified amount of time from now to modify a chunk in a buffer. Params: buffer-name 'modifications' time-delay { <  module,  priority,  output, time-in-ms > }."),
    ("schedule-event-after-module", "Create an event to occur after the next event for the named module. Params: after-module action {< params, module, destination, maintenance, output, details, precondition, delay, include-maintenance, dynamic > }"),
    ("add-dm", "Create chunks in the current model and add them to the model's declarative memory. Params: {'chunk-description'}*."),
    ("add-dm-fct", "Create chunks in the current model and add them to the model's declarative memory. Params: ({'chunk-description'}*)."),
    ("define-chunks", "Create chunks in the current model. Params: [ 'chunk-description'* | chunk-name* ]."),
    ("define-chunks-fct", "Create chunks in the current model from a single list of chunk descriptions. Params: (chunk-description*)."),
    ("define-chunk-spec", "Create a chunk-spec and return its id. Params: '{{mod} slot value}'*."),
    ("chunk-type", "Create a new chunk type or print all chunk-types to command trace. Params: [ nil | ([name | (name (/:include/ parent)*)] {'doc-string'} [ slot | (slot 'default-value') ]*)]"),
    ("dm", "Print the representation of the chunks from declarative memory with the given names.  Params: chunk-name*"),
    ("sdm", "Print the chunks from declarative memory which match with the given specification.  Params: 'spec*' where spec ::= {modifier} slot-name slot-value"),
    ("pprint-chunks", "Print the indicated chunks. Params: chunk-name*."),
    ("pprint-chunks-fct", "Print the indicated chunks. Params: (chunk-name*)."),
    ("print-dm-finsts", "Print a table showing any finsts marking recently retrieved chunks in declarative memory. No params."),
    ("print-activation-trace", "Prints out the activation trace from the saved data at the specified time. Params: time {ms?}"),
    ("print-chunk-activation-trace", "Prints out the activation trace from the saved data for a specific chunk at the specified time. Params: chunk time {ms?}"),
    ("saved-activation-history", "Returns a list of the lists of times and chunks for which activation data has been recorded. No params."),
    ("act-r-version", "Returns the current ACT-R version string. No params."),
    ("all-productions", "Returns a list of all the production names in the current model. No params."),
    ("buffers", "Return a list with all the currently defined buffers' names. Params {sorted}."),
    ("chunks", "Returns a list of all the currently defined chunks. No params."),
    ("model-buffers", "Return a list with all the currently defined buffers' names from the current model. Params {sorted}."),
    ("multi-buffer-p", "Check if the named buffer is a multi-buffer. Params: buffer"),
    ("get-m-buffer-chunks", "Return a list of possible chunks for specified multi-buffer. Params: buffer-name."),
    ("used-production-buffers", "Returns a list of all the buffers which are used in the productions of the current model. No params."),
    ("chunk-filled-slots-list", "Returns the list of slot names for the chunk provided which can be in an arbitrary or cannonical ordering. Params: chunk-name {cannonical-order?}."),
    ("printed-chunk", "Return the text output of printing a chunk optionally with the parameter values. Params: chunk-name {show-parameters?}."),
    ("dm-slot-filters", "Returns a list of the possible filters (lists of slot names) for chunks in DM for the Environment to use. No params."),
    ("filter-dm-chunks", "Returns a list of chunks in DM which match the specified slot list filter. Params: list-of-slots."),
    ("dm-chunk-details", "Returns the text of the declarative parameters and printed representaion of a chunk for the Environment. Params: chunk-name."),
    ("dm-whynot-text", "Returns the text of the whynot-dm information for a chunk for use in the Environment. Params: chunk-name."),
    ("production-details", "Returns the text of the procedural parameters and printed representaion of a production for the Environment. Params: production-name."),
    ("whynot-text", "Returns the text of the whynot information for a production for use in the Environment. Params: production-name."),
    ("printed-visicon", "Return a string with the printed output of all of the attributes of the features in the model's visicon and if the optional parameter is given as true also show any pending updates. Params: {show_pending}."),
    ("printed-audicon", "Return the string containing all the printed attributes of the features in the model's audicon. No params."),
    ("printed-buffer-chunk", "Return a string of the printed output for the contents of specified buffers. Params: {buffer-name}*."),
    ("printed-buffer-status", "Return a string of the printed output for the status of specified buffers. Params: {buffer-name}*."),
    ("production-graph", "Return the production transition graph as a list of nodes and counted edges from the recorded production history. No params."),
    ("production-graph-all", "Return production-history graph coordinates for all production transitions. No params."),
    ("production-graph-freq", "Return production-history graph coordinates showing production transition frequencies. No params."),
    ("production-graph-cycles", "Return production-history graph coordinates showing production transition cycles. No params."),
    ("production-graph-u-cycles", "Return production-history graph coordinates showing unique production transition cycles. No params."),
    ("production-graph-runs", "Return production-history graph coordinates showing production transition runs. No params."),
    ("production-graph-u-runs", "Return production-history graph coordinates showing unique production transition runs. No params."),
    ("production-graph-utility", "Return production-history graph coordinates showing production utilities. No params."),
    ("sorted-module-names", "Returns a list of the names of all current modules sorted alphabetically. No params."),
    ("modules-with-parameters", "Returns a list of the names of all current modules which provided parameters sorted alphabetically. No params."),
    ("modules-parameters", "Returns a list of the names of all the parameters for the named module sorted alphabetically. Params: module-name."),
    ("printed-parameter-details", "Returns a string of the parameter output for the given parameter's value and details. Params: parameter-name."),
    ("whynot", "Print the production matching details for specified productions. Params: production-name*."),
    ("whynot-dm", "Print the declarative memory matching information based on the last retrieval request. Params: chunk-name*."),
    ("print-conflict-tree", "Print the current model's procedural conflict tree to the command trace. No params."),
    ("simulate-retrieval-request", "Given the specification of a retrieval request, output the activation trace of what would happen if that request were made now. Params: 'request-details*'."),
    ("mp-show-queue", "Print out a description of the events currently scheduled to occur optionally indicating which will show in the trace. Params: {mark-traced?}"),
    ("mp-queue-text", "Return the text contianing a description of the events currently scheduled to occur optionally indicating which will show in the trace. Params: {mark-traced?}"),
    ("chunk-p", "Returns whether the given name is the name of a chunk in the model. Params: chunk-name."),
    ("chunk-copied-from", "Returns the name of the chunk from which the given chunk was copied. Params: chunk-name."),
    ("chunk-slot-value", "Return the value of a slot in a chunk. Params: chunk-name slot-name"),
    ("set-chunk-slot-value", "Change the value of a slot in a chunk. Params: chunk-name slot-name 'new-value'"),
    ("copy-chunk", "Returns the name of a chunk which is a copy of the chunk name provided. Params: chunk-name."),
    ("delete-chunk", "Delete a chunk from the model. Params: chunk-name."),
    ("purge-chunk", "Delete a chunk from the model and release the name. Params: chunk-name."),
    ("act-r-random", "Return a random number up to limit using the current model's random stream. Params: limit."),
    ("permute-list", "Return a randomly ordered copy of the list provided using the current model's random stream. Params: list."),
    ("get-time", "Get current absolute model time or relative real time in milliseconds. Params: {model}."),
    ("release-chunk-spec-id", "Release the chunk-spec associated with the provided id. Params: chunk-spec-id."),
    ("add-command", "Add a command to the ACT-R dispatcher. Params: name underlying-function documentation single-instance local-name encoded."),
    ("remove-command", "Remove a command from the ACT-R dispatcher. Params: name."),
    ("monitor-command", "Arrange for a monitor command to be called before or after the monitored command. Params: act-r-command monitor {when}."),
    ("remove-command-monitor", "Remove a monitor command from a monitored command. Params: command-name monitor."),
    ("check", "Check whether a command is registered. Params: name. Returns true, whether the caller owns it, and its documentation, or nil when the command does not exist."),
    ("record-history", "Start recording the named history information if not already being recorded. Params: history-name*"),
    ("stop-recording-history", "Stop recording the named history information if all requested starts have stopped. Params: history-name*"),
    ("history-data-available", "Return whether there is any data available for the indicated history stream. Params: history-name {param*}"),
    ("start-incremental-history-data", "Returns an id for incrementally acquiring the history data from the specfied file or current data using the indicated parameters if no file given in segements roughly the length specified. Params: processor-name byte-size-suggestion {file-name-or-nil} {additional-parameters}*"),
    ("get-incremental-history-data", "Return the next segment from the history data associated with the given id or an empty list if there is no more data. Params: id"),
    ("save-history-data", "Save the current history data from the indicated history stream and parameters to the specifed file. Params: history-name file-name {additional-parameters}*"),
    ("predict-bold-response", "Return and possibly print the BOLD data prediction for the currently recorded buffer trace. Params: {start-time {end-time {output-results}}}"),
    ("sgp", "Set or get the current model's parameters. Params: parameter-name {value}*."),
    ("spp", "Set or get production parameters. Params: see ACT-R manual."),
    ("sdp", "Set or get declarative parameters. Params: see ACT-R manual."),
    ("no-output", "Turn off the command output while executing the command provided. Params: command-string {parameter-for-command*}"),
    ("pp", "Prints the internal representation of the productions with the given names or all if none provided. Params: p-name*."),
    ("p", "Define a production. Params: name {documentation} lhs ==> rhs."),
    ("delete-production", "Delete the named production from the current model. Params: production-name."),
    ("add-visicon-features", "Add visual features to the current model's visicon. Params: 'feature-spec'*."),
    ("modify-visicon-features", "Change a visual feature in the current model's visicon. Params: 'visicon-mod-spec'*."),
    ("delete-visicon-features", "Remove visual features from the current model's visicon. Params: feature-name*."),
    ("delete-all-visicon-features", "Remove all visual features from the current model's visicon. No params."),
    ("print-visicon", "Print all the attributes of the features in the model's visicon and also changes that have happened but have not been processed yet. No params."),
    ("set-visual-center-point", "Set the default reference point about which the vision module determines clockwise and couterclockwise. Params: x y"),
    ("new-tone-sound", "Create a new pure tone sound event for the audio module. Params: frequency duration {onset-time {onset-time-in-milliseconds}}."),
    ("new-ongoing-sound", "Create a new custom sound event for the audio module which continues until stopped with end-ongoing-sound. Params: 'content' delay recode {onset-time {'location' {'kind' {time-in-milliseconds {'extra-features'*}}}}}."),
    ("end-ongoing-sound", "Stop a sound that was created with new-ongoing-sound. Params: id."),
    ("print-audicon", "Print all the attributes of the features in the model's audicon to command output. No params."),
    ("new-word-sound", "Create a new word sound event for the audio module. Params: word {onset-time {'location' {time-in-milliseconds}}}."),
    ("new-digit-sound", "Create a new digit sound event for the audio module. Params: digit {onset-time {onset-time-in-milliseconds}}."),
    ("get-articulation-time", "Get the time required to speak a string of text in seconds or milliseconds. Params: string {milliseconds?}."),
    ("register-articulation-time", "Set the time required to speak a string of text in seconds or milliseconds. Params: string time {milliseconds?}."),
    ("get-syllable-count", "Return the syllable count for a word in the lookup table. Params: word"),
    ("register-syllable-count", "Add the syllable count for a word to the lookup table used when :use-syllable-table is true. Params: word syllables"),
    ("remove-syllable-count", "Remove the syllable count for a word in the lookup table used when :use-syllable-table is true. Params: word"),
    ("pbreak", "Pbreak causes the scheduler to stop when the specified productions are selected. Params: production-name*."),
    ("punbreak", "Punbreak removes the break flag from the specified productions. Params: production-name*."),
    ("trigger-reward", "Provide a reward signal for utility learning. Params: reward-value {maintenance?}"),
    ("pdisable", "Prevent the specified productions from being selected. Params: production-name*."),
    ("penable", "Restore the specified productions which were disabled. Params: production-name*."),
    ("correlation", "Compute the correlation between two lists of numbers. Params: (results) (data) {output}."),
    ("mean-deviation", "Compute the RMSD between two lists of numbers. Params: (results) (data) {output}."),
    ("clear-exp-window", "Remove all items from the window provided. Params: {window}."),
    ("add-text-to-exp-window", "Create a text item for the provided experiment window with the features specified and place it in the window. Params: window text {< x, y, color, height, width, font-size >}."),
    ("add-line-to-exp-window", "Create a line for the provided experiment window with the features specified and then place it in that window. Params: window (x1 y1) (x2 y2) {color}."),
    ("add-button-to-exp-window", "Create a button item for the provided experiment window with the features specified and place it in the window. Params: window {< text, x, y, action, height, width, color >}."),
    ("add-image-to-exp-window", "Create an image item for the provided experiment window with the features specified and place it in the window. Params: window text file {< x, y, width, height, action, clickable >}."),
    ("create-image-for-exp-window", "Create an image item for the provided experiment window with the features specified. Params: window text file {< x, y, width, height, action, clickable >}."),
    ("add-items-to-exp-window", "Add the given items to the window provided. Params: window {item}*."),
    ("remove-items-from-exp-window", "Remove the given items from the window provided. Params: window {item}*."),
    ("modify-text-for-exp-window", "Modify a text item created for an experiment window with the new features provided. Params: text-item {< text, x, y, color, height, width, font-size >}."),
    ("modify-line-for-exp-window", "Change the attributes of a line that was created for an experiment window. Params: line (x1 y1) (x2 y2) {color}."),
    ("open-exp-window", "Create a window for a task. Params: title {< visible, width, height, x, y >}"),
    ("close-exp-window", "Close the indicated experiment window or if none provided the only open one. Params: {window}"),
    ("close-all-exp-windows", "Close all the currently open experiment windows. No params."),
    ("select-exp-window", "Bring an open experiment window to the front. Params: {window}."),
    ("add-virtual-window-handler", "Add a handler to be notified of AGI visible virtual window updates. Params: interface-cmd {display-handler?}."),
    ("remove-virtual-window-handler", "Remove a handler from AGI visible virtual window updates. Params: interface-cmd."),
    ("visible-virtual-window-mouse-click", "Report a real mouse click interaction with a visible virtual window. Params: window-title click-x-pos click-y-pos."),
    ("install-device", "Command to install a device for an interface. Params: (interface device {device-detail})"),
    ("process-events", "Allow the process to handle pending events and yield to other threads. No params."),
    ("visible-virtuals-available?", "Check whether there is a handler active for displaying visible virtual windows."),
    ("start-hand-at-mouse", "Have the model place its right hand on the mouse before running. No parameters"),
    ("output-key", "Command called when a key on the virtual keyboard is pressed by the model or a real key is pressed in a visible experiment window which can be monitored.  It is passed 2 parameters: model-name and key-name. Should not be called directly."),
    ("output-speech", "Command called when the model performs a speech action which can be monitored.  It is passed 2 parameters: model-name and the text spoken. Should not be called directly."),
    ("release-key", "Command called when a key on the virtual keyboard is released by the model which can be monitored.  It is passed 2 parameters: model-name and key-name. Should not be called directly."),
    ("model-trace", "Output of the ACT-R model for monitoring - shouldn't be evaluated. Params: model-output-string."),
    ("command-trace", "Output of the ACT-R commands for monitoring - shouldn't be evaluated. Params: command-output-string."),
    ("warning-trace", "Output of the ACT-R warning commands for monitoring - shouldn't be evaluated. Params: warning-string."),
    ("general-trace", "Output from the act-r-output command for monitoring - shouldn't be evaluated. Params: command-output-string."),
];

/// Documentation for a built-in command, if one is registered.
pub fn builtin_doc(name: &str) -> Option<&'static str> {
    BUILTIN_DOCS
        .iter()
        .find_map(|(n, doc)| (*n == name).then_some(*doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::session::BUILTINS;
    use std::collections::BTreeSet;

    #[test]
    fn every_builtin_has_documentation() {
        for name in BUILTINS {
            let doc = builtin_doc(name).unwrap_or_else(|| panic!("no documentation for {name}"));
            assert!(!doc.trim().is_empty(), "empty documentation for {name}");
        }
    }

    #[test]
    fn documentation_names_match_builtins() {
        let names: BTreeSet<&str> = BUILTIN_DOCS.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names.len(),
            BUILTIN_DOCS.len(),
            "duplicate documentation entries"
        );
        for name in &names {
            assert!(
                BUILTINS.contains(name),
                "documentation for unknown command {name}"
            );
        }
    }

    #[test]
    fn add_dm_documents_declarative_memory() {
        let doc = builtin_doc("add-dm").expect("add-dm documentation");
        assert!(doc.contains("declarative memory"), "{doc:?}");
    }
}
