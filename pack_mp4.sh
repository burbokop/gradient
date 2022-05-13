#!/bin/bash

pad_num=`seq -f "%04g" $1 $1`
echo "pad_num: $pad_num"
ffmpeg -framerate 24 -i ./out/video_$pad_num/frame%00d.ppm -c:v libx264 -profile:v high -crf 20 -pix_fmt yuv420p ./out/output_$pad_num.mp4
