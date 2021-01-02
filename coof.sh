mpv --input-ipc-server=/tmp/mpv-socket --scripts=./coof.js --video-sync=display-resample --pause=no https://stream.wotos.eu/snw_master.m3u8 &
sleep 2
./coof http://stream.wotos.eu/spreader /tmp/mpv-socket
