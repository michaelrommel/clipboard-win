## Support for image copy and paste

### Overview

The windows clipboard can host multiple different image formats in
parallel. Selection of a particular content is by format_id, which is
either a predefined constant like CF_BITMAP or CF_DIBV5 or otherwise by
iterating over the format_name as a string.

The following table provides an overview, which application populates the
clipboard with which format. The format name is not standardized, so multiple
possibilities might have to be tried, e.g. PNG or image/png. In the table the
"best" format is stated, so if an application provides CF_BITMAP and CF_DIBV5,
the table shows the latter, because requesting this format may result in the
best image quality. The crate will always request `CF_DIBV5` for Bmp formats
as Windows itself will convert the internally available format for us.

The Rust `image` crate supports all the binary input formats listed in the
header. Svg and Text can be handled with text im- and exports.


### Application support

| Application   | Bmp                         | Png                      | Jpeg       | Gif | Ico       | WebP       | Svg           | Text           |
| ------------- | :-------------------------: | :----------------------: | :--------: | :-: | :-------: | :--------: | :-----------: | :------------: |
| Snipping Tool | CF_DIBV5 <br /> (no alpha)  | -                        | -          | -   | -         | -          | -             | -              |
| Snip & Sketch | CF_BITMAP <br /> (no alpha) | PNG <br /> (alpha)       | -          | -   | -         | -          | -             | -              |
| Greenshot     | CF_DIB <br /> (no alpha)    | PNG <br /> (alpha)       | -          | -   | -         | -          | -             | -              |
| Paint 3D      | CF_BITMAP <br /> (no alpha) | PNG <br /> (alpha)       | -          | -   | -         | -          | -             | -              |
| CopyQ         | CF_DIBV5 <br /> (alpha)     | image/png <br /> (alpha) | image/jpeg | -   | image/ico | image/webp | -             | -              |
| Word          | CF_BITMAP <br /> (alpha)    | PNG <br /> (alpha)       | JFIF       | GIF | -         | -          | image/svg+xml | -              |
| Powerpoint    | CF_BITMAP <br /> (alpha)    | PNG <br /> (alpha)       | JFIF       | GIF | -         | -          | image/svg+xml | -              |
| Inkscape      | -                           | PNG <br /> (alpha)       | -          | -   | -         | -          | image/svg+xml | -              |
| DrawIO        | -                           | -                        | -          | -   | -         | -          | -             | CF_UNICODETEXT |
| Chrome / Edge | CF_DIBV5 <br /> (alpha)     | PNG <br /> (alpha)       | -          | -   | -         | -          | -             | -              |
| Firefox       | CF_DIBV5 <br /> (no alpha)  | -                        | -          | -   | -         | -          | -             | -              |


### Format selection

Based on this table, the implementation should cater for the following use
cases:

- Specifying the desired clipboard content based on the `image`-crate's
  ImageFormat Enum.
- Specifying the desired output format as `image`-crate's ImageOutputFormat
- Specifying the `Format Name` as string for the text formats, the outputformat
  is automatically set to `CF_UNICODETEXT`

