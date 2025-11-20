--- A collection of types to be included / used in other Lua files.
---
--- These types are either required by the Lua API or required for the normal
--- operation of this Lua plugin.
---

---@alias GoToPosition 
---| '"beginning"' # Beginning of the file
---| "end" # End of the note 
---| "header" # Header position
local POSITION = {
  beginning = "start", 
  ending = "end",
  header = "header"
}

---@alias Key string Key of a note

---@class ztl.Note
--- Note entry 
---@field id Key
---    The unique identifier of the note
---@field header string
---    Header of note
---@field kind string?
---    Optional descriptor of the note kind (book, theorem, definition, etc.)

---@class ztl.NoteSpan
---	Span information of a note associated with a file (used for display)
---@field target Key Note target
---@field header string Note header duplicate
---@field kind string? Note kind
---@field outgoing table<string, ztl.NoteOutgoing>
