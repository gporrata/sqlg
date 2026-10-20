# Requests

Behavior requested for sqlg. The original app remains described in `docs/SPEC.md`.

## Window

- Dragging the title bar moves the window. Tabs and their buttons still act as tabs.
- Dragging an edge or a corner resizes the window.

## Schema

- Schema names and table names are visible in the schema column.

## Results

- A null cell is blank.
- Result columns can be resized, and those widths are restored with the layout.
- One result fills the results area. Several results stay stacked.
- Each result can show or hide columns. The choices are grouped by the table alias in the query, and the list can select all or clear all.
- Column types are distinguished by color wherever a type is shown.

## Layout

- The layout choices are icons.
- One layout keeps the schema column, with the results under the editor.
- One layout hides the schema column, with the results under the editor.
- The chosen layout and the pane sizes are restored the next time the app opens.
- The dividers between panes can be dragged along their full length, and they look like the splitters in Visual Studio Code.

## Editor

- Quitting asks to save only when the editor text was opened from a file and then changed.
- Text that is not linked to a file is kept and comes back on the next launch.
- Quitting does not write a linked file on its own.
- Selected text stays readable.

## SQL checking

- A warning or error is shown in full, and its text stays readable.
- A name introduced in the select list counts as that name where SQL allows it, including `having`. A valid use is not reported as an unknown column.

## Connections

- A saved password is reused without asking for the keychain password again.
- The footer identifies the connection by its name and host. The username and password stay off the footer.

## Appearance

- A settings control opens the list of themes. There is no separate light and dark switch.
- Choosing a theme applies it immediately, with no save step, and the choice is restored later.
- The list is a familiar set of editor themes, and it includes One Dark Pro Night Flat.
- The theme controls stay reachable while the window can still be resized.

## Icon

- The app icon is the database icon from the earlier Electron app.
