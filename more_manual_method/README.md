# Using Aria2 for very fast downloads

So there are many other feeds than the full disc GOES iamges available from GOES, but I had already finished my rust implementation for just those images.

Instead of spending time on building the right abstraction, I reached for my favorite cli based tools.

These steps get you the sun observation images which are taken very frequently (like every few minutes) and look extremely cool.

# Steps

Navigate to one of the directory listings in a browser. One of them is "https://cdn.star.nesdis.noaa.gov/GOES16/SUVI/FD/Fe093/" and save the html output with ctrl-s
We do this because it's annoying to get absolute urls without some scripting and firefox seems to just spit them all out in absolute form.

Run this sed script to extract the high res jpegs.

```shell
sed -n '/-1200x1200.jpg/ {s/^.*"\(.*\)".*$/\1/p}' WHATEVER_YOU_NAMED_THE_SAVED_SITE.html > dl_list.txt
```

## Alternate: awk one-liner that probably works

```shell
BASE="https://cdn.star.nesdis.noaa.gov/GOES16/SUVI/FD/Fe093/"; curl "${BASE}" | awk -v base="${BASE}" \
  'match($0, /href="([^"]*-1200x1200\.jpg)"/, m) { print base m[1] }' > dl_list.txt
```


## Alternate: htmlq

Really quick with [htmlq](https://github.com/mgdm/htmlq)

```shell
BASE="https://cdn.star.nesdis.noaa.gov/GOES16/SUVI/FD/Fe093/"; curl "${BASE}" | htmlq --base="${BASE}" --attribute href a | grep -- "-1200x1200.jpg" > dl_list.txt
```


Download with aria2c

**Warning** This will probably be several gigabytes of image data and will stretch back about a month of images.

```shell
aria2c --max-connection-per-server=16 --max-concurrent-downloads=150 --continue=true --log aria2.log --log-level=notice --console-log-level=error --file-allocation=falloc -i dl_list.txt
```

Output as a high speed timelapse with a filesize appropriate for something like Discord.

``shell
ffmpeg -framerate 150 -pattern_type glob -i "*.jpg" -vf "format=yuv420p" -c:v libx264 -crf 23 -pix_fmt yuv420p output150_hq.mp4
```

# Ideas for future enhancement

Optical flow between frames to make the timelapse better and the resulting video file smaller such that I can bump up the quality.
