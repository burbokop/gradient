#!/bin/bash

pad_num=`seq -f "%04g" $1 $1`
echo "pad_num: $pad_num"

# coping audio from $2 to $1
ffmpeg -i ./out/output_$pad_num.mp4 -i $2 -c copy -map 0:0 -map 1:1 -shortest ./out/output_audio_$pad_num.mp4