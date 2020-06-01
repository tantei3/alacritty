// Super WIPy sixel parser
// Works in the few test cases I tried at least

#[derive(Copy, Clone, Debug)]
pub struct HeaderParser {
    macro_param: usize,
    transparent_bg: bool,
    grid_size: usize,
    aspect_numerator: usize,
    aspect_denominator: usize,
    xsize: usize,
    ysize: usize,
    state: HeaderParserState
}

impl HeaderParser {
    fn new() -> HeaderParser {
        HeaderParser {
            macro_param: 0,
            transparent_bg: true,
            grid_size: 0,
            aspect_numerator: 2,
            aspect_denominator: 1,
            xsize: 0,
            ysize: 0,
            state: HeaderParserState::FormattingDone
        }
    }

    fn put(&mut self, byte: u8) {
        use HeaderParserState::*;

        match (self.state, byte) {
            // (MacroFormatter, b';') => {
            //     self.state = BgSelect;
            // },
            // (BgSelect, b'0') => {}
            // (BgSelect, b'1') => {
            //     self.transparent_bg = true;
            // }
            // (BgSelect, b';') => {
            //     self.state = HGridSize;
            // }
            // (_, b'q') => {
            //     self.state = FormattingDone;
            // }
            (FormattingDone, b'"') => {
                self.state = AspectNumerator;
            }
            (AspectNumerator, b'0'..=b'9') => {
                self.aspect_numerator = (byte - 48) as usize;
            }
            (AspectNumerator, b';') => {
                self.state = AspectDenominator;
            }
            (AspectDenominator, b'0'..=b'9') => {
                self.aspect_denominator = (byte - 48) as usize;
            }
            (AspectDenominator, b';') => {
                self.state = HorizontalExtent;
            }
            (HorizontalExtent, b'0'..=b'9') => {
                self.xsize *= 10;
                self.xsize += (byte - 48) as usize;
            }
            (HorizontalExtent, b';') => {
                self.state = VerticalExtent;
            }
            (VerticalExtent, b'0'..=b'9') => {
                self.ysize *= 10;
                self.ysize += (byte - 48) as usize;
            }
            (_, _) => {
                self.state = Done;
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum HeaderParserState {
    MacroFormatter,
    BgSelect,
    HGridSize,
    FormattingDone,
    AspectNumerator,
    AspectDenominator,
    HorizontalExtent,
    VerticalExtent,
    Done
}



#[derive(Copy, Clone, Debug)]
pub struct ColorParser {
    slot: usize,
    rgb: (u8, u8, u8),
    hls: bool,
    state: ColorParserState
}

impl ColorParser {
    fn new() -> ColorParser {
        ColorParser {
            slot: 0,
            rgb: (0, 0, 0),
            hls: false,
            state: ColorParserState::Slot
        }
    }

    fn put(&mut self, byte: u8) -> ColorParserState {
        use ColorParserState::*;
        match (self.state, byte) {
            (Slot, b'0'..=b'9') => {
                self.slot *= 10;
                self.slot += byte as usize - 48 as usize;
            }
            (Slot, b';') => {
                self.state = ColorSpace;
            }
            (ColorSpace, b'0'..=b'9') => {
                self.hls = false;
            }
            (ColorSpace, b';') => {
                self.state = Red;
            }
            (Red, b'0'..=b'9') => {
                self.rgb.0 *= 10;
                self.rgb.0 += byte - 48;
            }
            (Red, b';') => {
                self.state = Green;
            }
            (Green, b'0'..=b'9') => {
                self.rgb.1 *= 10;
                self.rgb.1 += byte - 48;
            }
            (Green, b';') => {
                self.state = Blue;
            }
            (Blue, b'0'..=b'9') => {
                self.rgb.2 *= 10;
                self.rgb.2 += byte - 48;
            }
            (Blue, _) => {
                let r = self.rgb.0 as u32 * 255 / 100;
                self.rgb.0 = r as u8;
                let g = self.rgb.1 as u32 * 255 / 100;
                self.rgb.1 = g as u8;
                let b = self.rgb.2 as u32 * 255 / 100;
                self.rgb.2 = b as u8;
                self.state = NewColor(self.rgb);
            }
            (Slot, _) => {
                self.state = SetSlot(self.slot);
            }
            (_, _) => {
                self.state = Invalid;
            }
        }

        self.state
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ColorParserState {
    Slot,
    ColorSpace,
    Red,
    Green,
    Blue,
    NewColor((u8, u8, u8)),
    SetSlot(usize),
    Invalid
}


pub struct SixelDecoder {
    rgb: Vec<u8>,
    colors: Vec<Color>,
    parser: SixelParserState,
    color: usize,
    xpos: usize,
    ypos: usize,
    pub xsize: usize,
    pub ysize: usize,
    repeat: usize,
}

fn wut() {
    dbg!("wut");
}

impl SixelDecoder {
    pub fn new() -> SixelDecoder {
        SixelDecoder {
            rgb: vec![],
            colors: Vec::with_capacity(256),
            parser: SixelParserState::Header(HeaderParser::new()),
            color: 0,
            xpos: 0,
            ypos: 0,
            xsize: 0,
            ysize: 0,
            repeat: 0,
        }
    }

    pub fn current_color(&mut self) -> Color {
        while self.colors.len() <= self.color {
            self.colors.push(Color::default());
        }
        self.colors[self.color]
    }

    pub fn current_sixel<'a, 'b: 'a>(&'b mut self) -> Sixel<'a, impl Iterator<Item=&'a mut [u8]>> {
        let pixels = self.rgb
                         .chunks_exact_mut(3)
                         .skip(self.xpos)
                         .skip(self.ypos * self.xsize)
                         .step_by(self.xsize)
                         .take(6);
        Sixel::new(pixels)
    }

    fn dbg_rgb(&self) {
        dbg!("current rgb:");
        for line in self.rgb.chunks_exact(3) {
            dbg!(format!("{}, {}, {}",
                         line[0],
                         line[1],
                         line[2]));
        }
    }

    pub fn into_rgb(self) -> Vec<u8> {
        self.rgb
    }

    pub fn put(&mut self, byte: u8) {
        match (&mut self.parser, byte) {
            (SixelParserState::ColorParser(cp), _) => {
                use ColorParserState::*;
                cp.put(byte);

                match cp.state {
                    NewColor((r,g,b)) => {
                        let c = Color::new(r,g,b);

                        self.colors.push(c);
                        self.parser = SixelParserState::Default;
                        self.put(byte);
                    }
                    SetSlot(n) => {
                        self.color = n;
                        self.parser = SixelParserState::Default;
                        self.put(byte);
                    }
                    Invalid => {
                        self.parser = SixelParserState::Default;
                        self.put(byte);
                    }
                    _ => {}
                }
            }
            (SixelParserState::Repeat, b'0'..=b'9') => {
                self.repeat *= 10;
                self.repeat += (byte - 48) as usize;
                self.parser = SixelParserState::Repeat;
            }
            (SixelParserState::Header(hp), _) => {
                hp.put(byte);

                match hp.state {
                    HeaderParserState::Done => {
                        self.xsize = hp.xsize;
                        self.ysize = hp.ysize;

                        self.rgb.reserve(self.xsize * self.ysize);
                        self.rgb.resize_with(self.xsize * self.ysize * 3, Default::default);

                        self.parser = SixelParserState::Default;

                        self.put(byte);
                    }
                    _ => {}
                }
            }
            (_, b'"') => {
                self.parser = SixelParserState::Header(HeaderParser::new());
            }
            (_, b'#') => {
                self.parser = SixelParserState::ColorParser(ColorParser::new());
            }
            (_, b'!') => {
                self.parser = SixelParserState::Repeat;
            }
            (_, b'-') => {
                self.xpos = 0;
                self.ypos += 6;
            }
            (_, b'$') => {
                self.xpos = 0;
            }
            (_, b'?'..=b'~') => {
                if self.xpos == self.xsize {
                    self.xpos = 0;
                    self.parser = SixelParserState::Default;
                }

                let color =  self.current_color();

                let mut sixel = self.current_sixel();
                sixel.paint(byte, color);
                drop(sixel);

                self.xpos += 1;


                if self.repeat > 1 {
                    for _ in 1..self.repeat {
                        {
                            let mut sixel = self.current_sixel();
                            sixel.paint(byte, color);
                        }
                        self.xpos += 1;
                    }
                    self.repeat = 0;
                }

                self.repeat = 0;
                self.parser = SixelParserState::Default;
            }
            (_, _) => {
                dbg!("unhandled!");
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum SixelParserState {
    Default,
    Header(HeaderParser),
    ColorParser(ColorParser),
    Repeat,
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Default for Color {
    fn default() -> Color {
        Color {
            r: 0,
            g: 0,
            b: 0
        }
    }
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Color {
        let mut color = Color::default();
        color.r = r;
        color.g = g;
        color.b = b;
        color
    }

    fn paint_pixel(&self, pixel: &mut Pixel) {
        pixel.rgb[0] = self.r;
        pixel.rgb[1] = self.g;
        pixel.rgb[2] = self.b;
    }
}


pub struct Pixel<'a> {
    rgb: &'a mut [u8]
}

impl<'a> Pixel<'a> {
    pub fn new(buf: &'a mut [u8]) -> Pixel<'a> {
        Pixel {
            rgb: buf
        }
    }
}



impl<'a> std::fmt::Debug for Pixel<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(fmt,
               "{}, {}, {}",
               self.rgb[0],
               self.rgb[1],
               self.rgb[2])?;
        Ok(())
    }
}



#[derive(Debug)]
pub struct Sixel<'a, I>
where
    I: Iterator<Item=&'a mut [u8]>
{
    pixels: I
}

impl<'a, I> Sixel<'a, I>
where
    I: Iterator<Item=&'a mut [u8]>
{
    fn new(buf: I) -> Sixel<'a, I> {
        Sixel {
            pixels: buf
        }
    }

    fn paint(&mut self, byte: u8, color: Color) {
        let bitmask = decode_strip(byte);

        for mask in &bitmask {
            let pixel = match self.pixels.next() {
                Some(pixel) => pixel,
                None => return
            };

            if *mask {
                color.paint_pixel(&mut Pixel::new(pixel));
            }
        }
    }
}

fn decode_strip(byte: u8) -> [bool; 6] {
    let byte = byte - 63;
    let a = (byte & (1 << 0)) > 0;
    let b = (byte & (1 << 1)) > 0;
    let c = (byte & (1 << 2)) > 0;
    let d = (byte & (1 << 3)) > 0;
    let e = (byte & (1 << 4)) > 0;
    let f = (byte & (1 << 5)) > 0;

    [a, b, c, d, e, f]
}
