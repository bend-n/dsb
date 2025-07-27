// |jlfJKlBM
// |||||||||||||||||||||||||||||||||||||
#![allow(incomplete_features)]
#![feature(
    super_let,
    debug_closure_helpers,
    const_trait_impl,
    anonymous_lifetime_in_impl_trait,
    deref_patterns,
    generic_const_exprs,
    guard_patterns,
    impl_trait_in_bindings,
    if_let_guard,
    import_trait_associated_functions
)]
use std::iter::zip;
pub mod cell;
use atools::Join;
use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::shape::cluster::Glyph;
use swash::text::cluster::{CharCluster, Parser, Token};
use swash::text::{Codepoint, Script};
use swash::zeno::Format;

pub struct Fonts<'a, 'b, 'c, 'd> {
    regular: FontRef<'a>,
    bold: FontRef<'b>,
    italic: FontRef<'c>,
    bold_italic: FontRef<'d>,
}
impl<'a, 'b, 'c, 'd> Fonts<'a, 'b, 'c, 'd> {
    pub fn new(
        regular: impl Into<FontRef<'a>>,
        bold: impl Into<FontRef<'b>>,
        italic: impl Into<FontRef<'c>>,
        bold_italic: impl Into<FontRef<'d>>,
    ) -> Self {
        Self {
            regular: regular.into(),
            bold: bold.into(),
            italic: italic.into(),
            bold_italic: bold_italic.into(),
        }
    }
}
#[implicit_fn::implicit_fn]
pub unsafe fn render(
    cells: &[Cell],
    (c, r): (usize, usize),
    ppem: f32,
    bgcolor: [u8; 3],
    fonts: Fonts,
    line_spacing: f32,
) -> Image<Box<[u8]>, 3> {
    assert_eq!(c * r, cells.len(), "cells too short.");
    let met = fonts.regular.metrics(&[]);
    let fac = ppem / met.units_per_em as f32;

    let (fw, fh_) = dims(&fonts.regular, ppem, line_spacing);
    let fh = fh_;
    let (w, h) = (
        (fw * c as f32).ceil() as u32,
        (fh_ * r as f32).ceil() as u32,
    );
    let mut i = Image::build(w as _, h as _).fill(bgcolor);
    if w < 60 || h < 60 {
        return i;
    }
    for (col, k) in cells.chunks_exact(c as _).zip(0..) {
        for (&cell, j) in zip(col, 0..) {
            if cell.style.bg != bgcolor {
                let cell = Image::<_, 4>::build(fw.ceil() as u32, (fh_).round() as u32)
                    .fill(cell.style.bg.join(255));

                unsafe {
                    i.as_mut().overlay_at(
                        &cell,
                        (j as f32 * fw).round() as u32,
                        (((k) as f32 * fh) + ((k as f32) * line_spacing as f32)).round() as u32,
                    )
                };
            }
        }
    }
    let mut characters: Vec<Glyph> = vec![Glyph::default(); c];

    for (col, k) in cells.chunks_exact(c as _).zip(0..) {
        let tokenized = col.iter().enumerate().map(|(i, cell)| {
            let ch = cell.letter.unwrap_or(' ');
            (
                Token {
                    ch,
                    offset: i as u32,
                    len: ch.len_utf8() as u8,
                    info: ch.properties().into(),
                    data: i as u32,
                },
                cell.style.flags,
            )
        });

        macro_rules! input {
            ($rule:expr, $font:expr) => {
                let mut scx = ShapeContext::new();
                let mut shaper = scx
                    .builder($font)
                    .size(ppem)
                    .script(Script::Latin)
                    .features([
                        ("ss19", 1),
                        ("ss01", 1),
                        ("ss20", 1),
                        ("liga", 1),
                        ("rlig", 1),
                    ])
                    .build();

                let mut cluster = CharCluster::new();

                let mut parser =
                    Parser::new(Script::Latin, tokenized.clone().filter($rule).map(|x| x.0));
                while parser.next(&mut cluster) {
                    cluster.map(|ch| $font.charmap().map(ch));
                    shaper.add_cluster(&cluster);
                }
                shaper.shape_with(|x| {
                    x.glyphs.into_iter().for_each(|x| {
                        characters[x.data as usize] = *x;
                    })
                });
            };
        }
        input!(|x| x.1 & Style::ITALIC == 0, fonts.regular);
        input!(|x| x.1 & Style::ITALIC != 0, fonts.bold_italic); // bifont is an instance of ifont

        for (&cell, glyph) in characters.iter().map(|x| (&col[x.data as usize], x)) {
            let j = glyph.data as usize;
            let mut color = cell.style.color;
            if (cell.style.flags & Style::DIM) != 0 {
                color = color.map(|x| x / 2);
            }
            // if (cell.style.flags & Style::UNDERLINE) != 0 {
            //     unsafe {
            //         i.as_mut().overlay_at(
            //             &Image::<_, 4>::build(fw.ceil() as u32, 2).fill(color.join(255)),
            //             4 + (j as f32 * fw as u32,
            //             (k as f32 * (ppem * 1.25)) as u32 + 5,
            //         )
            //     };
            // }
            // if (cell.style.flags & Style::STRIKETHROUGH) != 0 {
            //     unsafe {
            //         i.as_mut().overlay_at(
            //             &Image::<_, 4>::build(fw.ceil() as u32, 2).fill(color.join(255)),
            //             4 + (j as f32 * fw) as u32,
            //             (k as f32 * (ppem * 1.25)) as u32 - 5,
            //         )
            //     };
            // }
            if let Some(_) = cell.letter {
                let f = match cell.style.flags {
                    f if f & (Style::BOLD | Style::ITALIC) != 0 => fonts.bold_italic,
                    f if f & Style::BOLD != 0 => fonts.bold,
                    f if f & Style::ITALIC != 0 => fonts.italic,
                    _ => fonts.regular,
                };
                let id = glyph.id;
                let mut scbd = ScaleContext::new();
                let mut scbd = scbd.builder(f);
                scbd = scbd.size(ppem);

                let x = Render::new(&[Source::Outline])
                    .format(Format::Alpha)
                    .render(&mut scbd.build(), id)
                    .unwrap();
                unsafe {
                    if x.placement.width == 0 {
                        continue;
                    }
                    let item = Image::<_, 1>::build(x.placement.width, x.placement.height)
                        .buf_unchecked(x.data);
                    // item.as_ref().show();
                    dbg!(x.placement);

                    i.as_mut().blend_alpha_and_color_at(
                        &item.as_ref(),
                        color,
                        ((j as f32 * fw + glyph.x) + x.placement.left as f32).round() as u32,
                        // fh as u32 - x.placement.top as u32,
                        ((((k + 1) as f32 * (fh)) - (x.placement.top as f32))
                            + ((k as f32) * line_spacing as f32))
                            .round() as u32
                            - (met.descent * fac).round() as u32,
                    );
                }
            }
        }
    }

    // if x.view_o == Some(x.cells.row) || x.view_o.is_none() {
    //     let cell =
    //         Image::<_, 4>::build(3, (ppem * 1.25).ceil() as u32).fill([0xFF, 0xCC, 0x66, 255]);
    //     unsafe {
    //         i.as_mut().overlay_at(
    //             &cell,
    //             4 + ((x.cursor.0 - 1) as f32 * sz) as u32,
    //             (x.cursor.1 as f32 * (ppem * 1.25)) as u32 - 20,
    //         )
    //     };
    // }
    i
}

pub fn dims(font: &FontRef, ppem: f32, line_spacing: f32) -> (f32, f32) {
    // ppem * (m.max_width / m.units_per_em as f32),
    // m.max_width * (ppem / m.units_per_em as f32);
    let m = font.metrics(&[]);
    let f = ppem / m.units_per_em as f32;
    (
        m.max_width * f,
        (m.ascent + m.descent) * f, // + line_spacing * f,
    )
}
pub use crate::cell::Cell;
use crate::cell::Style;
use fimg::{Image, OverlayAt};
