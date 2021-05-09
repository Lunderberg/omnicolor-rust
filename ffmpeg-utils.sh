#!/bin/bash

# Command-line testing, implemented in GrowthImage.
function generate_video_from_png() {
    # -f image2pipe                 # inputs interpreted as a series of images
    # -i -                          # Accept input on stdin
    # -framerate 24                 # output framerate
    # -y                            # overwrite output file
    # -hide_banner -loglevel error  # Suppress output
    # -crf 23                       # Quality level, 0-51.  0 is lossless.

    cat temp-dir/*.png | \
        ffmpeg -f image2pipe -i - \
               -framerate 24 \
               -vcodec libx264 \
               -preset fast \
               -crf 23 \
               -pix_fmt yuv420p \
               -y output.mp4
}


# Command-line testing, not implemented in GrowthImage.  File size is
# huge compared to the mp4 output (factor of 10-ish).
function generate_gif_from_png() {
    cat temp-dir/*.png | \
        ffmpeg -f image2pipe -i - \
               -framerate 24 \
               -vf "scale=320:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" \
               -y output.gif
}


# Post-processing utility, stack multiple videos into one
function merge_videos() {
    # Based on https://superuser.com/a/1100429/507487
    ffmpeg -i output.mp4 -i palette.mp4 \
           -filter_complex '[1][0]scale2ref[2nd][ref];[ref][2nd]vstack' \
           -vcodec libx264 -crf 23 -preset fast \
           -y knot-side-by-side.mp4
}

merge_videos
