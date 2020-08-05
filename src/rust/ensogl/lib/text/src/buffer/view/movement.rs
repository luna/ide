//! Text cursor transform implementation.

use super::*;
use crate::buffer::data;
use crate::buffer::data::unit::*;
use crate::buffer::view::word::WordCursor;
use crate::buffer::view::selection;



// =================
// === Transform ===
// =================

/// Selection transformation patterns. Used for the needs of keyboard and mouse interaction.
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Transform {
    /// Select all text.
    All,
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
    /// Move to the left selection border. Cursors will not be modified.
    LeftSelectionBorder,
    /// Move to the right selection border. Cursors will not be modified.
    RightSelectionBorder,
    /// Move to the left by one word.
    LeftWord,
    /// Move to the right by one word.
    RightWord,
    /// Select the word at every cursor.
    Word,
    /// Move to left end of visible line.
    LeftOfLine,
    /// Move to right end of visible line.
    RightOfLine,
    /// Move up one visible line.
    Up,
    /// Move down one visible line.
    Down,
    /// Move to the start of the document.
    StartOfDocument,
    /// Move to the end of the document
    EndOfDocument,
}



// ==========================
// === Transform Handling ===
// ==========================

impl ViewBuffer {
    /// Convert selection to caret location after a vertical movement.
    fn vertical_motion_selection_to_location
    (&self, selection:Selection, move_up:bool, modify:bool) -> Location {
        let end = selection.end;
        if modify {end} else if move_up {selection.min()} else {selection.max()}
    }

    /// Compute movement based on vertical motion by the given number of lines.
    fn vertical_motion
    (&self, selection:Selection, line_delta:Line, modify:bool) -> selection::Shape {
        let move_up      = line_delta < 0.line();
        let location     = self.vertical_motion_selection_to_location(selection,move_up,modify);
        let min_line     = 0.line();
        let max_line     = self.last_line_index();
        let border_step  = if move_up { -1.line() } else { 1.line() };
        let snap_top     = location.line < min_line;
        let snap_bottom  = location.line > max_line;
        let next_line    = max_line + border_step;
        let bottom       = location.line + line_delta;
        let line         = if snap_top {border_step} else if snap_bottom {next_line} else {bottom};
        let tgt_location = location.with_line(line);
        selection::Shape(selection.start,tgt_location)
    }


    pub fn line_offset_of_location_X2(&self, location:Location) -> Option<Bytes> {
        if location.line < 0.line() {
            return Some(0.bytes())
        }
        let mut column = 0.column();
        let mut offset = self.byte_offset_from_line_index(location.line).ok()?;
        let max_offset = self.end_byte_offset_from_line_index(location.line).ok()?;
        while column < location.column {
            match self.next_grapheme_offset(offset) {
                None => break,
                Some(off) => {
                    column += 1.column();
                    offset = off;
                }
            }
        }
        Some(offset.min(max_offset))
    }

    /// Apply the movement to each region in the selection, and returns the union of the results.
    ///
    /// If `modify` is `true`, the selections are modified, otherwise the results of individual region
    /// movements become carets. Modify is often mapped to the `shift` button in text editors.
    pub fn moved_selection(&self, movement: Transform, modify: bool) -> selection::Group {
        let mut result = selection::Group::new();
        for &selection in self.selection.borrow().iter() {
            let new_selection = self.moved_selection_region(movement, selection, modify);
            result.merge(new_selection);
        }
        result
    }

//    pub fn selection_after_insert(&self, bytes: Bytes) -> selection::Group {
//        let mut result = selection::Group::new();
//        let mut offset = bytes;
//        for &selection in self.selection.borrow().iter() {
//            let new_selection = selection.map(|t| t + offset);
//            offset += bytes;
//            result.add(new_selection);
//        }
//        result
//    }

    pub fn prev_grapheme_location(&self, location:Location) -> Option<Location> {
        let offset      = self.line_col_to_offset(location)?;
        let prev_offset = self.prev_grapheme_offset(offset);
        let out = prev_offset.map(|off| self.offset_to_location(off));
        out
    }

    pub fn next_grapheme_location(&self, location:Location) -> Option<Location> {
        let offset      = self.line_col_to_offset(location)?;
        let next_offset = self.next_grapheme_offset(offset);
        next_offset.map(|off| self.offset_to_location(off))
    }

    /// Compute the result of movement on one selection region.
    pub fn moved_selection_region
    (&self, movement:Transform, region:Selection, modify:bool) -> Selection {
        let text        = &self.data();
        let shape       = |start,end| selection::Shape(start,end);
        let shape : selection::Shape = match movement {
            Transform::All               => shape(default(),self.offset_to_location(text.byte_size())),
            Transform::Up                => self.vertical_motion(region, -1.line(), modify),
            Transform::Down              => self.vertical_motion(region,  1.line(), modify),
            Transform::StartOfDocument   => shape(region.start,default()),
            Transform::EndOfDocument     => shape(region.start,self.offset_to_location(text.byte_size())),

            Transform::Left => {
                let def     = shape(region.start,default());
                let do_move = region.is_caret() || modify;
                if  do_move { self.prev_grapheme_location(region.end).map(|t|shape(region.start,t)).unwrap_or(def) }
                else        { shape(region.start,region.min()) }
            }

            Transform::Right => {
                let def     = shape(region.start,region.end);
                let do_move = region.is_caret() || modify;
                if  do_move { self.next_grapheme_location(region.end).map(|t|shape(region.start,t)).unwrap_or(def) }
                else        { shape(region.start,region.max()) }
            }

            Transform::LeftSelectionBorder => {
                shape(region.start,region.min())
            }

            Transform::RightSelectionBorder => {
                shape(region.start,region.max())
            }

            Transform::LeftOfLine => {
                let end = Location(region.end.line,0.column());
                shape(region.start,end)
            }

            Transform::RightOfLine => {
                let line             = region.end.line;
                let text_byte_size   = text.byte_size();
                let is_last_line     = line == self.last_line_index();
                let next_line_offset = self.byte_offset_from_line_index(line+1.line()).unwrap();
                let offset           = if is_last_line { text_byte_size } else {
                    text.prev_grapheme_offset(next_line_offset).unwrap_or(text_byte_size)
                };
                let end = self.offset_to_location(offset);
                shape(region.start,end)
            }

            Transform::LeftWord => {
                let end_offset      = self.line_col_to_offset(region.end).unwrap_or_default();
                let mut word_cursor = WordCursor::new(text,end_offset);
                let offset          = word_cursor.prev_boundary().unwrap_or(0.bytes());
                let end             = self.offset_to_location(offset);
                shape(region.start,end)
            }

            Transform::RightWord => {
                let end_offset      = self.line_col_to_offset(region.end).unwrap_or_default();
                let mut word_cursor = WordCursor::new(text,end_offset);
                let offset          = word_cursor.next_boundary().unwrap_or_else(|| text.byte_size());
                let end             = self.offset_to_location(offset);
                shape(region.start,end)
            }

            Transform::Word => {
                let end_offset      = self.line_col_to_offset(region.end).unwrap_or_default();
                let mut word_cursor = WordCursor::new(text,end_offset);
                let offsets         = word_cursor.select_word();
                let start           = self.offset_to_location(offsets.0);
                let end             = self.offset_to_location(offsets.1);
                shape(start,end)
            }
        };
        let start = if modify { shape.start } else { shape.end };
        let end   = shape.end;
        Selection(start,end,region.id)
    }
}
