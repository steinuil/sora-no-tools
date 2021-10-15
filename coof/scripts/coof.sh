mpv --input-ipc-server=/tmp/mpv-socket --scripts=./coof.js --video-sync=display-resample --pause=no https://stream.wotos.eu/snw_master.m3u8 &
sleep 2
./coof --server http://stream.wotos.eu/spreader --mpv-socket /tmp/mpv-socket
