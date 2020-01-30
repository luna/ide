//! Rendering TextField.

pub mod assignment;
#[allow(missing_docs)]
pub mod selection;

use crate::prelude::*;

use crate::display::object::DisplayObjectData;
use crate::display::shape::glyph::font::FontRegistry;
use crate::display::shape::glyph::font::FontRenderInfo;
use crate::display::shape::glyph::system::GlyphSystem;
use crate::display::shape::text::content::TextFieldContent;
use crate::display::shape::text::content::TextFieldContentFullInfo;
use crate::display::shape::text::cursor::Cursor;
use crate::display::shape::text::cursor::Cursors;
use crate::display::shape::text::render::assignment::GlyphLinesAssignment;
use crate::display::shape::text::render::assignment::LineFragment;
use crate::display::shape::text::render::selection::SelectionSpritesGenerator;
use crate::display::shape::primitive::def::*;
use crate::display::shape::primitive::system::ShapeSystem;
use crate::display::symbol::geometry::compound::sprite::Sprite;

use nalgebra::Vector2;
use nalgebra::Vector3;
use crate::display::shape::text::TextFieldProperties;
use crate::display::world::World;


// =======================
// === RenderedContent ===
// =======================

type GlyphLine = crate::display::shape::glyph::system::Line;

/// Structure containing sprites bound to one cursor with its selection.
#[derive(Debug)]
pub struct CursorSprites {
    /// Cursor sprite.
    pub cursor: Sprite,
    /// Selection sprites.
    pub selection: Vec<Sprite>,
}

/// Structure with all data and sprites required for rendering specific TextField.
#[derive(Debug)]
pub struct RenderedContent {
    /// System used for rendering glyphs.
    pub glyph_system: GlyphSystem,
    /// System used for rendering cursors.
    pub cursor_system: ShapeSystem,
    /// System used for rendering selections.
    pub selection_system: ShapeSystem,
    /// All drawn glyph lines.
    pub glyph_lines: Vec<GlyphLine>,
    /// All drawn cursors..
    pub cursors: Vec<CursorSprites>,
    /// Current assignment of glyph lines to actual lines of text.
    pub assignment: GlyphLinesAssignment,
    /// line height in pixels.
    pub line_height: f32,
    /// Display object of the whole rendered content.
    pub display_object: DisplayObjectData,
}


// === Construction ===

impl RenderedContent {

    /// Create RenderedContent structure.
    pub fn new(world:&World, properties:&TextFieldProperties, fonts:&mut FontRegistry) -> Self {
        let line_height       = properties.text_size;
        let window_size       = properties.size;
        let color             = properties.base_color;
        let font              = fonts.get_render_info(properties.font_id);
        let cursor_system     = Self::create_cursor_system(world,line_height);
        let selection_system  = Self::create_selection_system(world);
        let cursors           = Vec::new();
        let mut glyph_system  = GlyphSystem::new(world,properties.font_id);
        let display_object    = DisplayObjectData::new(Logger::new("RenderedContent"));
        display_object.add_child(&selection_system);
        display_object.add_child(&glyph_system);
        display_object.add_child(&cursor_system);

        let assignment        = Self::create_assignment_structure(window_size,line_height,font);
        let glyph_lines_count = assignment.glyph_lines_count();
        let length            = assignment.max_glyphs_in_line;
        let bsl_start         = Vector2::new(0.0, 0.0);

        let indexes     = 0..glyph_lines_count;
        let glyph_lines = indexes.map(|_| {
            glyph_system.new_empty_line(bsl_start,line_height,length,color)
        }).collect();
        RenderedContent {glyph_system,cursor_system,selection_system,glyph_lines,cursors,
            line_height,display_object,assignment}
    }

    fn create_cursor_system(world:&World,line_height:f32) -> ShapeSystem {
        const WIDTH_FUNCTION:&str = "fract(input_time / 1000.0) < 0.5 ? 2.0 : 0.0";
        let cursor_definition     = SharpRect(WIDTH_FUNCTION,line_height);
        ShapeSystem::new(world,&cursor_definition)
    }

    fn create_selection_system(world:&World) -> ShapeSystem {
        const ROUNDING:f32 = 3.0;
        let width          = "input_size.x";
        let height         = "input_size.y";
        let r              = ROUNDING;
        let selection_definition = RoundedRectByCorner(width,height,r,r,r,r);
        ShapeSystem::new(world,&selection_definition)
    }

    fn create_assignment_structure
    ( window_size : Vector2<f32>
    , line_height : f32
    , font        : &mut FontRenderInfo
    ) -> GlyphLinesAssignment {
        // Display_size.(x/y).floor() makes space for all lines/glyph that fit in space in
        // their full size. But we have 2 more lines/glyph: one clipped from top or left, and one
        // from bottom or right.
        const ADDITIONAL:usize = 2;
        let displayed_lines    = (window_size.y / line_height).floor() as usize + ADDITIONAL;
        let space_width        = font.get_glyph_info(' ').advance;
        let displayed_chars    = (window_size.x / space_width).floor();
        // This margin is to ensure, that after x scrolling we won't need to refresh all the lines
        // at once.
        let x_margin           = (displayed_lines as f32) * line_height / space_width;
        let max_glyphs_in_line = (displayed_chars + 2.0 * x_margin).floor() as usize + ADDITIONAL;
        GlyphLinesAssignment::new(displayed_lines,max_glyphs_in_line,x_margin,line_height)
    }
}


// === DisplayObject ===

impl From<&RenderedContent> for DisplayObjectData {
    fn from(rendered_content:&RenderedContent) -> Self {
        rendered_content.display_object.clone_ref()
    }
}


// === Update ===

impl RenderedContent {
    /// Update all displayed glyphs.
    pub fn update_glyphs(&mut self, content:&mut TextFieldContent, fonts:&mut FontRegistry) {
        let glyph_lines       = self.glyph_lines.iter_mut().enumerate();
        let lines_assignment  = glyph_lines.zip(self.assignment.glyph_lines_fragments.iter());
        let assigned_lines    = lines_assignment.filter_map(|(l,opt)| opt.as_ref().map(|f|(l,f)));
        let dirty_lines       = std::mem::take(&mut content.dirty_lines);
        let dirty_glyph_lines = std::mem::take(&mut self.assignment.dirty_glyph_lines);
        for ((index,glyph_line),fragment) in assigned_lines {
            if dirty_glyph_lines.contains(&index) || dirty_lines.is_dirty(fragment.line_index) {
                let mut f_content = content.full_info(fonts);
                let bsl_start     = Self::baseline_start_for_fragment(fragment,&mut f_content);
                let line          = &content.lines[fragment.line_index];
                let chars         = &line.chars()[fragment.chars_range.clone()];
                glyph_line.set_baseline_start(bsl_start);
                glyph_line.replace_text(chars.iter().cloned(),fonts);
            }
        }
    }

    /// Update all displayed cursors with their selections.
    pub fn update_cursors(&mut self, cursors:&Cursors, content:&mut TextFieldContentFullInfo) {
        let cursor_system = &self.cursor_system;
        self.cursors.resize_with(cursors.cursors.len(),|| Self::new_cursor_sprites(cursor_system));
        for (sprites,cursor) in self.cursors.iter_mut().zip(cursors.cursors.iter()) {
            let position = Cursor::render_position(&cursor.position,content);
            sprites.cursor.set_position(Vector3::new(position.x,position.y,0.0));
            sprites.cursor.size().set(Vector2::new(2.0,self.line_height));

            let selection = cursor.selection_range();
            let line_height   = self.line_height;
            let system        = &self.selection_system;
            let mut generator = SelectionSpritesGenerator {content,line_height,system};
            sprites.selection = generator.generate(&selection);
        }
    }

    fn baseline_start_for_fragment(fragment:&LineFragment, content:&mut TextFieldContentFullInfo)
    -> Vector2<f32> {
        let mut line = content.line(fragment.line_index);
        if fragment.chars_range.start >= line.chars().len() {
            line.baseline_start()
        } else {
            let x = line.get_char_x_position(fragment.chars_range.start);
            let y = line.baseline_start().y;
            Vector2::new(x,y)
        }
    }

    fn new_cursor_sprites(cursor_system:&ShapeSystem) -> CursorSprites {
        CursorSprites {
            cursor    : cursor_system.new_instance(),
            selection : Vec::new(),
        }
    }
}
