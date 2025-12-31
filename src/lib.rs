#![allow(incomplete_features)]
#![feature(
    bigint_helper_methods,
    proc_macro_hygiene,
    portable_simd,
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
#![allow(unsafe_op_in_unsafe_fn)]
use std::iter::{successors, zip};
pub mod cell;
use atools::prelude::*;
use fimg::{Image, OverlayAt};
use itertools::Itertools;
use lru_cache::LruCache;
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::shape::cluster::Glyph;
use swash::text::cluster::{CharCluster, Parser, Token};
use swash::text::{Codepoint, Script};
use swash::zeno::Format;
use swash::{FontRef, Instance};
use umath::FF32;

pub use crate::cell::Cell;
use crate::cell::Style;
pub struct Fonts<'a, 'b, 'c, 'd> {
    pub regular: F<'a>,
    pub bold: F<'b>,
    pub italic: F<'c>,
    pub bold_italic: F<'d>,

    cache: LruCache<(u8, FF32, u16), swash::scale::image::Image>,
    shape_cache: LruCache<Vec<Cell>, Vec<(u16, f32)>>,
    scx: ShapeContext,
}
#[derive(Clone, Copy)]
pub enum F<'a> {
    FontRef(FontRef<'a>, &'static [(u32, f32)]),
    Instance(FontRef<'a>, Instance<'a>),
}
impl<'a, T: Into<FontRef<'a>>> From<T> for F<'a> {
    fn from(value: T) -> Self {
        Self::FontRef(value.into(), &[])
    }
}

impl<'a> std::ops::Deref for F<'a> {
    type Target = FontRef<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            F::FontRef(font_ref, _) | F::Instance(font_ref, _) => font_ref,
        }
    }
}
impl<'a> F<'a> {
    pub fn instance(of: impl Into<FontRef<'a>>, i: Instance<'a>) -> Self {
        Self::Instance(of.into(), i)
    }
    pub fn variations(&self) -> Box<dyn Iterator<Item = (u32, f32)> + 'a> {
        match self {
            F::FontRef(_, x) => Box::new(x.into_iter().copied()),
            F::Instance(font_ref, instance) => Box::new(
                instance
                    .values()
                    .zip(font_ref.variations())
                    .map(|x| (x.1.tag(), x.0)),
            ),
        }
    }
}

impl<'a, 'b, 'c, 'd> Fonts<'a, 'b, 'c, 'd> {
    pub fn new(
        regular: impl Into<F<'a>>,
        bold: impl Into<F<'b>>,
        italic: impl Into<F<'c>>,
        bold_italic: impl Into<F<'d>>,
    ) -> Self {
        Self {
            regular: regular.into(),
            bold: bold.into(),
            italic: italic.into(),
            bold_italic: bold_italic.into(),
            cache: LruCache::new(3000),
            shape_cache: LruCache::new(2000),
            scx: ShapeContext::new(),
        }
    }
}
pub unsafe fn render_owned(
    cells: &[Cell],
    (c, r): (usize, usize),
    ppem: f32,
    fonts: &mut Fonts,
    line_spacing: f32,
    subpixel: bool,
) -> Image<Box<[u8]>, 3> {
    let (w, h) = size(&fonts.regular, ppem, line_spacing, (c, r));
    let mut i = Image::build(w as _, h as _).fill([255; 3]);
    render(
        cells,
        (c, r),
        ppem,
        fonts,
        line_spacing,
        subpixel,
        i.as_mut(),
        (0, 0),
    );
    i
}
#[implicit_fn::implicit_fn]
pub unsafe fn render(
    cells: &[Cell],
    (c, r): (usize, usize),
    ppem: f32,
    fonts: &mut Fonts,
    line_spacing: f32,
    subpixel: bool,
    mut i: Image<&mut [u8], 3>,
    (offset_x, offset_y): (u32, u32),
) {
    // assert_eq!(c * r, cells.len(), "cells too short.");

    let met = fonts.regular.metrics(&[]);
    let fac = ppem / met.units_per_em as f32;
    let (fw, fh_) = dims(&fonts.regular, ppem);
    let fh = fh_;
    // let (w, h) = (
    //     (fw * c as f32).ceil() as u32,
    //     height(&fonts.regular, ppem, line_spacing, r),
    // );
    for (col, k) in cells.chunks_exact(c as _).zip(0..) {
        for (&cell, j) in zip(col, 0..) {
            unsafe {
                /*i.as_mut().filled_box(
                     (
                        (j as f32 * fw).floor() as u32 // _
                            + offset_x,
                        (k as f32 * (fh + line_spacing * fac)).ceil() as u32 + offset_y
                    ),
                    fw.floor() as _, fh_.ceil() as _, cell.style.bg,
                );*/

                fill_in(
                    i.as_mut(),
                    (
                        (j as f32 * fw).floor() as u32 // _
                            + offset_x,
                        (k as f32 * (fh + line_spacing * fac)).ceil()
                            as u32
                            + offset_y,
                    ),
                    (fw.ceil() as _, fh_.ceil() as _),
                    cell.style.bg,
                );

                /*i.as_mut().clipping_overlay_at(
                    &cell,
                    (j as f32 * fw).floor() as u32 // _
                            + offset_x,
                    (k as f32 * (fh + line_spacing * fac)).floor() as u32
                        + offset_y,
                )*/
            };
        }
    }
    let mut characters: Vec<(u16, f32)> = vec![(0, 0.); c];

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
        let scx = &mut fonts.scx;
        let sch = &mut fonts.shape_cache;
        let mut cluster = CharCluster::new();
        macro_rules! input {
            ($rule:expr, $font:expr, $k: literal) => {
                tokenized
                    .clone()
                    .chunk_by($rule)
                    .into_iter()
                    .filter(|x| x.0)
                    .for_each(|(_, y)| {
                        let x = y.map(|x| x.0).collect::<Vec<_>>();
                        /*if let Some((s, d)) = sch.get_mut(&key) {
                            for (a, b) in d
                                .iter()
                                .zip(&mut characters[*s as usize..])
                            {
                                //characters[*c as usize] = (*a, *b);
                                *b = *a;
                            }
                            return;
                        }*/
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
                        let mut parser =
                            Parser::new(Script::Latin, x.into_iter());
                        while parser.next(&mut cluster) {
                            cluster.map(|ch| $font.charmap().map(ch));
                            shaper.add_cluster(&cluster);
                        }
                        let mut s = None;
                        let mut l = vec![];
                        shaper.shape_with(|x| {
                            s = s.or(Some(x.glyphs[0].data));
                            l.extend(
                                x.glyphs.into_iter().map(|x| (x.id, x.x)),
                            );

                            // println!("-");
                            x.glyphs.into_iter().for_each(|x| {
                                // dbg!(x.data);
                                characters[x.data as usize] = (x.id, x.x);
                            })
                        });
                    });
            };
        }
        let characters = if let Some(x) = fonts.shape_cache.get_mut(col) {
            &*x
        } else {
            input!(|x| x.1 & Style::ITALIC == 0, *fonts.regular, 0);
            input!(|x| x.1 & Style::ITALIC != 0, *fonts.italic, 1);
            fonts.shape_cache.insert(col.to_vec(), characters.clone());
            &characters
        };
        for (&cell, id, glyx, j) in characters
            .iter()
            .zip(0..)
            .map(|(&(id, x), index)| (&col[index], id, x, index))
        {
            let mut color = cell.style.color;
            if (cell.style.flags & Style::DIM) != 0 {
                color = color.map(|x| x / 2);
            }
            if (cell.style.flags & Style::UNDERLINE) != 0 {
                unsafe {
                    i.as_mut().overlay_at(
                        &Image::<_, 4>::build(
                            fw.ceil() as u32,
                            ((0.05 * ppem) as u32).max(1),
                        )
                        .fill(color.join(255)),
                        ((j as f32 * fw).round() as i32 + offset_x as i32)
                            .max(0) as u32,
                        (k as f32 * (fh + line_spacing * fac)
                            + (met.ascent * fac * 1.05))
                            .floor() as u32
                            + offset_y,
                    )
                };
            }
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
                let (key, f) = match cell.style.flags {
                    f if (f & Style::BOLD != 0)
                        & (f & Style::ITALIC != 0) =>
                        (3, fonts.bold_italic),
                    f if f & Style::BOLD != 0 => (2, fonts.bold),
                    f if f & Style::ITALIC != 0 => (1, fonts.italic),
                    _ => (0, fonts.regular),
                };

                let key = (key, FF32::new(ppem), id);
                if !fonts.cache.contains_key(&key) {
                    let mut scbd = ScaleContext::new();
                    let scbd = scbd
                        .builder(*f)
                        .variations(f.variations())
                        .hint(true)
                        .size(ppem);
                    let x = Render::new(&[Source::Outline])
                        .format(if subpixel {
                            Format::Subpixel
                        } else {
                            Format::Alpha
                        })
                        .render(&mut scbd.build(), id)
                        .unwrap();
                    fonts.cache.insert(key, x);
                };
                let x = fonts.cache.get_mut(&key).unwrap();

                if x.placement.width == 0 {
                    continue;
                }
                // println!("{:?} {:?}", glyph, x.placement);
                let x_ = (((j as f32 * fw + glyx + 0.125)
                    + x.placement.left as f32)
                    .round() as i32
                    + offset_x as i32)
                    .max(0) as u32;
                let y_ = (((((k + 1) as f32 * fh)
                    - (x.placement.top as f32))
                    .round() as i32
                    - (met.descent * fac).round() as i32
                    + (k as f32 * line_spacing as f32 * fac) as i32)
                    + offset_y as i32)
                    .max(0) as u32;
                if subpixel {
                    into(
                        i.as_mut(),
                        Image::build(
                            x.placement.width,
                            x.placement.height,
                        )
                        .buf(&x.data),
                        (x_, y_),
                        color.map(_ as _),
                    );
                } else {
                    i.as_mut().blend_alpha_and_color_at(
                        &Image::build(
                            x.placement.width,
                            x.placement.height,
                        )
                        .buf(&x.data),
                        color,
                        x_ as u32,
                        y_ as u32,
                    );
                }
            }
            // i.as_ref().show().save(j.to_string());
            // i.chunked_mut().for_each(|x| *x = [0; 3]);
        }
    }
    // successors(Some(fh), |&x| {
    //     (x + fh < 1832 as f32).then_some(x + fh + (line_spacing * fac))
    // })
    // .for_each(|x| {
    //     i.line((0, x as i32), (2000, x as i32), [255, 0, 255]);
    //     let x = x + line_spacing * fac;
    //     i.line((0, x as i32), (2000, x as i32), [255, 0, 255]);
    // });

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
}

fn blend(m: [u8; 3], c: [u8; 3], to: &mut [u8; 3]) {
    *to = [
        ((c[0] as u16 * m[2] as u16 + (255 - m[2] as u16) * to[0] as u16)
            / 255) as u8,
        ((c[1] as u16 * m[1] as u16 + (255 - m[1] as u16) * to[1] as u16)
            / 255) as u8,
        ((c[2] as u16 * m[0] as u16 + (255 - m[0] as u16) * to[2] as u16)
            / 255) as u8,
    ];
}

#[implicit_fn::implicit_fn]
#[unsafe(no_mangle)]
fn into(
    mut i: Image<&mut [u8], 3>,
    with: Image<&[u8], 4>,
    (x_, y_): (u32, u32),
    color: [u8; 3],
) {
    (0..with.height())
        .flat_map(|y| (0..with.width()).map(move |x| (x, y)))
        .zip(with.chunked())
        .for_each(|((x, y), d)| {
            i.get_pixel_mut(x.wrapping_add(x_), y.wrapping_add(y_)).map(
                |x| {
                    let mask = d.init();
                    blend(mask, color, x);
                },
            );
        })
}

pub fn fit(
    font: &FontRef,
    ppem: f32,
    line_spacing: f32,
    sizes: (usize, usize),
) -> (usize, usize) {
    let m = font.metrics(&[]);
    let f = ppem / m.units_per_em as f32;

    let (fw, fh) = dims(font, ppem);
    (
        (sizes.0 as f32 / fw).floor() as _,
        successors(Some(fh), |&x| {
            ((x + fh + line_spacing * f) < sizes.1 as f32)
                .then_some(x + fh + (line_spacing * f))
        })
        .count(),
    )
}

pub fn size(
    font: &FontRef,
    ppem: f32,
    line_spacing: f32,
    (c, r): (usize, usize),
) -> (usize, usize) {
    let (fw, fh) = dims(font, ppem);

    ((fw * c as f32).ceil() as usize, {
        let m = font.metrics(&[]);
        let f = ppem / m.units_per_em as f32;
        let ls = line_spacing * f;
        (((fh + ls) * r as f32).ceil() - ls) as usize
    })
}
#[unsafe(no_mangle)]
pub unsafe fn fill_in(
    mut image: Image<&mut [u8], 3>,
    (x1, y1): (u32, u32),
    (w, h): (u32, u32),
    with: [u8; 3],
) {
    let iw = image.width();
    for x in x1..1 + w + x1 {
        image.set_pixel(x, y1, with);
    }
    let from = y1 * iw + x1;
    let p = image.buffer_mut().as_mut_ptr();
    let n = w as usize * 3;
    let from = p.add(from as usize * 3);

    for y in y1 + 1..(y1 + h).min(image.height()) {
        core::ptr::copy(from, p.add(((y * iw + x1) * 3) as _), n);
        // image.buffer_mut().copy_within(from.clone(), ((y * iw + x1)*3) as _);
    }
}

pub fn dims(font: &FontRef, ppem: f32) -> (f32, f32) {
    let m = font.metrics(&[]);
    let f = ppem / m.units_per_em as f32;
    (m.max_width * f, (m.ascent + m.descent) * f)
}

// London. Michaelmas Term lately over, and the Lord Chancellor sitting in Lincoln’s Inn Hall.eImplacable November weather. As much mud in the streets, as if the waters had but newly retiredfrom the face of the earth,1 and it would not be wonderful to meet a Megalosaurus, forty feet longor so, waddling like an elephantine lizard up Holborn Hill.2 Smoke lowering down from chimneypots, making a soft black drizzle, with flakes of soot in it as big as full-grown snowflakes—gone intomourning, one might imagine, for the death of the sun.3 Dogs, undistinguishable in mire. Horses,scarcely better; splashed to their very blinkers. Foot passengers, jostling one another’s umbrellas,in a general infection of ill-temper, and losing their foothold at street-corners, where tens ofthousands of other foot passengers have been slipping and sliding since the day broke (if this dayever broke), adding new deposits to the crust upon crust of mud, sticking at those pointstenaciously to the pavement, and accumulating at compound interest.Fog everywhere. Fog up the river, where it flows among green aitsf and meadows; fog down theriver, where it rolls defiled among the tiers of shipping, and the waterside pollutions of a great (anddirty) city. Fog on the Essex marshes, fog on the Kentish heights. Fog creeping into the cabooses ofcollier-brigs; fog lying o ut on the yards, and hovering in the rigging of great ships; fog drooping onthe gunwalesg of barges and small boats. F og in the eyes and throats of ancient Greenwichpensioners, h wheezing by the firesides of their wards; fog in the stem and bowl of the afternoonpipe of the wrathful skipper, down in his close cabin; fog cruelly pinching the toes and fingers of hisshivering little ‘prentice boy on deck. Chance people on the bridges peeping over the parapets intoa nether sky of fog, with fog all round them, as if they were up in a balloon,4 and hanging in themisty clouds.Gas looming through the fog in divers places in the streets, much as the sun may, from the spongyfields, be seen to loom by husbandman and ploughboy. Most of the shops lighted two hours beforetheir time—as the gas seems to know, for it has a haggard and unwilling look.The raw afternoon is rawest, and the dense fog is densest, and the muddy streets are muddiest,near that leaden-headed old obstruction, appropriate ornament for the threshold of a leadenheaded old corporation: Temple Bar.5 And hard by Temple Bar, in Lincoln’s Inn Hall, at the very heartof the fog, sits the Lord High Chancellor i n his High Court of Chancery.Never can there come fog too thick, never can there come mud and mire too deep, to assort withthe groping and floundering condition which this High Court of Chancery, most pestilent of hoarysinners, holds, this day, in the sight of heaven and earth.On such an afternoon, if ever, the Lord High Chancellor ought to be sitting here—as here he iswith a foggy glory round his head, softly fenced in with crimson cloth and curtains, addressed by alarge advocate with great whiskers, a little voice, and an interminable brief,i and outwardlydirecting his contemplation to the lantern in the roof,j where he can see nothing but fog. On suchan afternoon, some score of members of the High Court of Chancery bar ought to be—as here theyare—mistily engaged in one of the ten thousand stages of an endless cause, tripping one anotherup on slippery precedents, groping knee-deep in technicalities, running their goat-hair and horsehair warded headsk against walls of words, and making a pretence of equity with serious faces, asplayers might. On such an afternoon, the various solicitors l in the cause, some two or three ofwhom have inherited it from  their fathers, who made a fortune by it, ought to be—as are they not—ranged in a line, in a long matted well (but you might look in vain for Truth at the bottom of it),6between the registrar’s red table and the silk gowns, with bills, cross-bills, answers, rejoinders,injunctions, affidavits, issues, references to masters,m masters’ reports, mountains of costlynonsense, piled before them. Well may the court be dim, with wasting candles here and there: wellmay the fog hang heavy in it, as if it would never get out; well may the stained glass windows losetheir colour, and admit no light of day into the place; well may the uninitiated from t he streets, whopeep in through the glass panes in the door, be deterred from entrance by its owlish aspect, and bythe drawl languidly echoing to the roof from the padded dais where the Lord High Chancellor looksinto the lantern that has no light in it, and where the attendant wigs are all stuck in a fog-bank! Thisis the Court of Chancery;7 which has its decaying houses and its blighted lands in every shire; whichhas its worn-out lunatic in every madhouse, and its dead in every churchyard; which has its ruinedsuitor, with his slipshod heels and threadbare dress, borrowing and begging through the round ofevery man’s acquaintance; which gives to monied might, the means abundantly of wearying out theright; which so exhausts finances, patience, courage, hope; so overthrows the brain and breaks theheart;8 that there is not an honourable man among its practitioners who would not give—who doesnot often give—the warning, ‘Suffer any wrong that can be done you, rather than come here!’9Who happen to be in the Lord Chancellor’s court this murky afternoon besides the Lord Chancellor,the counsel in the cause, two or three counsel who are never in any cause, and the well of solicitorsbefore mentioned? There is the registrar below the Judge, in wig and gown; and there are two orthree maces,n or petty-bags, or privy purses, or whatever they may be, in legal court suits. Theseare all yawning; for no crumb of amusement ever falls10 from JARNDYCE AND JARNDYCE (the causein hand), which was squeezed dry years upon years ago. The short-hand writers, the reporters ofthe court, and the reporters of the newspapers, 11 invariably decamp with the rest of the regularswhen Jarndyce and Jarndyce comes on. Their places are a blank. Standing on a seat at the side ofthe hall, the better to peer into the curtained sanctuary,o is a little mad old woman in a squeezedbonnet, who is always in court, from its sitting to its rising, and always expecting someincomprehensible judgment to be given in her favour. Some say she really is, or was, a party to asuit; but no one knows for certain, because no one cares. She carries some small litter in a reticulepwhich she calls her documents ; principally consisting of paper matchesq and dry lavender. A sal-low prisoner12 has come up, in custody, for the half-dozenth time, to make a personal application‘to purge himself of his contempt;’ which, being a solitary surviving executor who has fallen into astate of conglomeration about accounts of whi ch it is not pretended that he had ever anyknowledge, he is not at all likely ever to do. In the meantime his prospects in life are ended.Another ruined suitor, who periodically appears from Shropshire, and breaks out into efforts toaddress the Chancellor at the close of the day’s business, and who can by no means be made tounderstand that the Chancellor is legally ignorant of his existence r after making it desolate for aquarter of a century, plants himself in a good place and keeps an eye on the Judge, ready to call out‘My Lord!’ in a voice of sonorous complaint, on the instant of his rising. A few lawyers’ clerks andothers who know this suitor by sight, linger, on the chance of his furnishing some fun, andenlivening the dismal weather a little.Jarndyce and Jarndyce drones on. This scarecrow of a suit has, in course of time, become socomplicated, that no man alive knows what it means. The parties to it understand it least; but it hasbeen observed that no two Chancery lawyers can talk about it for five minutes, without coming to atotal disagreement as to all the premises. Innumerable children have been born into the cause;innumerable young people have married into it; innumerable old people have died out of it. Scoresof persons have deliriously found themselves made parties in Jarndyce and Jarndyce, withoutknowing how or why; whole families have inherited legendary hatreds with the suit. The littleplaintiff or defendant, who was promised a new rocking-horse when Jarndyce and Jarndyce shouldbe settled, has grown up, possessed himself of a real horse, and trotted away into the other world.Fair wards of courtshave faded into mothers and grandmothers; a long procession of Chancellorshas come in and gone out; the legion of bills in the suit have been transformed into mere bills ofmortality;t there are not three Jarndyces left upon the earth perhaps, since old Tom Jarndyce indespair blew his brains out at a coffee-house in Chancery Lane; but Jarndyce and Jarndyce stilldrags its dreary length before the Court,13 perennially hopeless.Jarndyce and Jarndyce has passed into a joke. That is the only good that has ever come of it. It hasbeen death to many, but it is a joke in the profession. Every master in Chancery has had a referenceout of it. Every Chancellor was ‘in it,’ for somebody or other, when he was counsel at the bar. Goodthings have been said about it by blue-nosed, bulbousshoed old benchers, u in select port-winecommittee after dinner in hall. Articled clerksv have been in the habit of fleshing their legal witupon it. The last Lord Chancellor handled it neatly when, correcting Mr. Blowers, the eminent silkgown who said that such a thing might happen when the sky rained potatoes,14 he observed, ‘orwhen we get through Jarndyce and Jarndyce, Mr. Blowers;‘—a pleasantry that particularly tickledthe maces, bags, and purses.How many people out of the suit, Jarndyce and Jarndyce has stretched forth its unwholesome handto spoil and corrupt, would be a very wide question. From the master, upon whose impaling filesreams of dusty warrants in Jarndyce and Jarndyce have grimly writhed into many shapes; down tothe copying-clerk in the Six Clerks’ Office,15 who has copied his tens of thousands of Chancery-foliopagesw under that eternal heading; no man’s nature has been made better by it. In trickery,evasion, procrastination, spoliation, botheration, under false pretences of all sorts, there areinfluences that can never come to good. The very solicitors’ boys who have kept the wretchedsuitors at bay, by protesting time out of mind that Mr. Chizzle, Mizzle, or otherwise, was particularlyengaged and h ad appointments until dinner, may have got an extra moral twist and shuffle intothemselves out of Jarndyce and Jarndyce. The receiver in the cause has acquired a goodly sum ofmoney by it, but has acquired too a distrust of his own mother; and a contempt for his own kind.Chizzle, Mizzle, and otherwise, have lapsed into a habit of vaguely promising themselves that theywill look into that outstanding little matter, and see what can be done for Drizzle—who was not wellused—when Jarndyce and Jarndyce shall be got out of the office. Shirking and sharking, in all theirmany varieties, have been sown broadcast by the ill-fated cause; and even those who havecontemplated its history from the outermost circle of such evil, have been insensibly tempted into aloose way of letting bad things alone to take their own bad course, and a loose belief that if theworld go wrong, it was, in some off-hand manner, never meant to go right.Thus, in the midst of the mud and at the heart of the fog, sits the Lord High Cha ncellor in his HighCourt of Chancery.‘Mr. Tangle,’ says the Lord High Chancellor, latterly something restless under the eloquence of thatlearned gentleman.‘Mlud,’ says Mr. Tangle. Mr. Tangle knows more of Jarndyce and Jarndyce than anybody. He isfamous for it—supposed never to have read anything else since he left school.‘Have you nearly concluded your argument?’‘Mlud, no—variety of points—feel it my duty tsubmit—ludship,’ is the reply that slides out of Mr.Tangle.‘Several members of the bar are still to be heard, I believe?’ says the Chancellor, with a slight smile.Eighteen of Mr. Tangle’s learned friends, each armed with a little summary of eighteen hundredsheets, bob up like eighteen hammers in a pianoforte, make eighteen bows, and drop into theireighteen places of obscurity‘We will proceed with the hearing on Wednesday fortnight,’ says the Chancellor. For the question atissue is only a question of costs, a mere bud on the forest tree of the parent suit, and really willcome to a settlement one of these days.The Chancellor rises; the bar rises; the prisoner is brought forward in a hurry; the man fromShropshire cries, ‘My lord!’ Maces, bags, and purses, indignantly proclaim silence, and frown at theman from Shropshire.‘In reference,’ proceeds the Chancellor, still on Jarndyce and Jarndyce, ‘to the young girl—‘‘Begludship’s pardon—boy,’ says Mr. Tangle, prematurely.‘In reference,’ proceeds the Chancellor, with extra distinctness, ‘to the young girl and boy, the twoyoung people,’ (Mr. Tangle crushed.)‘Whom I directed to be in attendance to-day, and who are now in my private room, I will see themand satisfy myself as to the expediency of making the order for their residing with their uncle.’Mr. Tangle on his legs again.‘Begludship’s pardon—dead.’‘With their,’ Chancellor looking through his double eye-glass at the papers on his desk, ‘grandfather.’‘Begludship’s pardon—victim of rash action—brains.’Suddenly a very little counsel, with a terrific bass voice, arises, fully inflated, in the back settlementsof the fog, and says, ‘Will your lordship allow me? I appear for him. He is a cousin, several timesremoved. I am not at the moment prepared to inform the Court in what exact remove he is acousin; but he is a cousin.’Leaving this address (delivered like a sepulchral message) ringing in the rafters of the roof, the verylittle counsel drops, and the fog knows him no more. Everybody looks for him. Nobody can seehim.16‘I will speak with both the young people,’ says the Chancellor anew, ‘and satisfy myself on thesubject of their residing with their cousin. I will mention the matter to-morrow morning when I takemy seat.’The Chancellor is about to bow to the bar, when the prisoner is presented. Nothing can possiblycome of the prisoner’s conglomeration, but his being sent back to prison; which is soon done. Theman from Shropshire ventures another demonstrative ‘My lord!’ but the Chancellor, being aware ofhim, has dexterously vanished. Everybody else quickly vanishes too. A battery of blue bagsx isloaded with heavy charges of papers and carried off by clerks; the little mad old woman marchesoff with her documents; the empty court is locked up. If all the injustice it has committed, and allthe misery it has caused, could only be locked up with it, and the whole burnt away in a greatfuneral pyre,—why so much the better for other parties than the parties in Jarndyce and Jarndyce!

#[test]
fn x() {
    pub static FONT: std::sync::LazyLock<FontRef<'static>> =
        std::sync::LazyLock::new(|| {
            FontRef::from_index(
                &include_bytes!("../../CascadiaCodeNF.ttf")[..],
                0,
            )
            .unwrap()
        });

    unsafe {
        let z = [
            Cell {
                style: Style {
                    bg: [31, 36, 48],
                    color: [255, 255, 255],
                    flags: Style::UNDERLINE,
                },
                letter: Some('['),
            },
            Cell {
                style: Style {
                    bg: [31, 36, 48],
                    color: [255, 173, 102],
                    flags: Style::UNDERLINE,
                },
                letter: Some('='),
            },
            Cell {
                style: Style {
                    bg: [31, 36, 48],
                    color: [204, 202, 194],
                    flags: Style::UNDERLINE,
                },
                letter: Some('s'),
            },
            Cell {
                style: Style {
                    bg: [31, 36, 48],
                    color: [255, 173, 102],
                    flags: Style::UNDERLINE,
                },
                letter: Some('>'),
            },
            Cell {
                style: Style {
                    bg: [31, 36, 48],
                    color: [255, 255, 255],
                    flags: 5,
                },
                letter: Some(']'),
            },
        ];
        let mut f = Fonts::new(*FONT, *FONT, *FONT, *FONT);
        render_owned(&z, (2, 2), 18.0, &mut f, 2.0, true);
        render_owned(&z, (2, 2), 18.0, &mut f, 2.0, true).show();
    }
}
