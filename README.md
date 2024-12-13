# Kewl Image Downloder for GOES satellites

This project was mostly a refresher on using clap CLI and parallelism in Rust.
The clap `-h` docs are pretty good.

# TODO

Add a quality option since there are higher and lower quality images available.

# Turning images into a video

`cd` into your destination directory and then use something like

```shell
$ ffmpeg -framerate 60 -pattern_type glob -i "*.jpg" -vf "format=yuv420p" -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p output_high_quality.mp4
```

to output a timelapse that preserves the resolution.
