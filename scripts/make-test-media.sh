#!/usr/bin/env bash
# Generates small test files into test-media/ (gitignored). Needs ffmpeg on PATH.
set -euo pipefail
out="$(dirname "$0")/../test-media"; mkdir -p "$out"; cd "$out"
ff() { ffmpeg -hide_banner -loglevel error -y "$@"; }

ff -f lavfi -i "sine=f=440:d=20" -ac 2 -ar 44100 -c:a flac -sample_fmt s16 \
   -metadata title="Tone 44.1" -metadata artist="Confluence Test" -metadata album="Test Tones" -metadata track=1 tone_44k16.flac
ff -f lavfi -i "sine=f=1000:d=20" -ac 2 -ar 96000 -c:a flac -sample_fmt s32 -bits_per_raw_sample 24 \
   -metadata title="Tone 96" -metadata artist="Confluence Test" -metadata album="Test Tones" -metadata track=2 tone_96k24.flac

printf '1\n00:00:01,000 --> 00:00:04,000\nHello from {\i1}Confluence Player{\i0}\n\n2\n00:00:05,000 --> 00:00:09,000\nSecond subtitle line\n' > subs.srt
printf ';FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=5000\ntitle=Opening\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=5000\nEND=12000\ntitle=Ending\n' > chapters.txt
ff -f lavfi -i "testsrc2=s=1920x1080:r=24:d=12" -f lavfi -i "sine=f=330:d=12" -f lavfi -i "sine=f=660:d=12" -i subs.srt -i chapters.txt \
   -map 0:v -map 1:a -map 2:a -map 3:s -map_metadata 4 -map_chapters 4 \
   -c:v libx265 -preset ultrafast -pix_fmt yuv420p10le -x265-params log-level=error \
   -c:a:0 flac -c:a:1 aac -b:a:1 192k -c:s ass \
   -metadata:s:a:0 language=eng -metadata:s:a:0 title="FLAC Stereo" -metadata:s:a:1 language=jpn -metadata:s:a:1 title="AAC" \
   -metadata:s:s:0 language=eng video_hevc10.mkv
rm subs.srt chapters.txt
ls -la

# A small tagged library: artist/album folders, cover art, mixed sample rates and codecs.
lib="Library"; rm -rf "$lib"
album() { # artist album year genre color1 color2 rate bits codec ntracks
  local dir="$lib/$1/$2"; mkdir -p "$dir"
  ff -f lavfi -i "gradients=s=600x600:c0=$5:c1=$6:x0=0:y0=0:x1=600:y1=600:d=1" -frames:v 1 "$dir/cover.png"
  for n in $(seq 1 "${10}"); do
    local ext=flac args=(-c:a flac -sample_fmt s32 -bits_per_raw_sample "$8")
    [ "$9" = alac ] && ext=m4a args=(-c:a alac -sample_fmt s32p)
    [ "$9" = mp3 ] && ext=mp3 args=(-c:a libmp3lame -b:a 320k)
    [ "$8" = 16 ] && [ "$9" = flac ] && args=(-c:a flac -sample_fmt s16)
    ff -f lavfi -t 30 -i "sine=f=$((220 * n)):sample_rate=$7" -ac 2 "${args[@]}" \
      -metadata title="$2 Part $n" -metadata artist="$1" -metadata album_artist="$1" -metadata album="$2" \
      -metadata date="$3" -metadata genre="$4" -metadata track="$n" "$dir/$(printf %02d "$n") $2 Part $n.$ext"
  done
}
album "Aurora Lane" "Northern Hours" 2021 "Ambient" 0x1e3c72 0x2a5298 96000 24 flac 5
album "Aurora Lane" "Glass Weather" 2023 "Ambient" 0xff9a8b 0xff6a88 44100 16 flac 4
album "The Midnight Cartographers" "Paper Maps" 2019 "Indie" 0x134e5e 0x71b280 48000 24 alac 4
album "Kiko Saraswati" "Sore di Jakarta" 2024 "Jazz" 0xf7971e 0xffd200 192000 24 flac 3
album "Kiko Saraswati" "Radio Edits" 2022 "Jazz" 0x8e2de2 0x4a00e0 44100 16 mp3 4
cp video_hevc10.mkv "$lib/Concert Clip.mkv"
find "$lib" -type f | wc -l

# synced lyrics sidecar (enhanced LRC: word timing + an instrumental break)
cat > "$lib/Aurora Lane/Northern Hours/01 Northern Hours Part 1.lrc" <<'LRC'
[ti:Northern Hours Part 1]
[ar:Aurora Lane]
[00:01.00]<00:01.00>Snow <00:01.60>falls <00:02.10>on <00:02.40>the <00:02.70>northern <00:03.40>hours<00:04.20>
[00:04.50]<00:04.50>Lanterns <00:05.20>hum <00:05.70>a <00:05.90>quiet <00:06.60>song<00:07.40>
[00:08.00]<00:08.00>Every <00:08.60>light <00:09.10>a <00:09.30>small <00:09.80>promise<00:10.80>
[00:11.00]
[00:18.00]<00:18.00>Morning <00:18.70>finds <00:19.20>us <00:19.60>here<00:20.60>
[00:21.50]<00:21.50>Still <00:22.00>listening<00:23.50>
LRC
