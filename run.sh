# ensure absolute dir
cd /mnt/raid/unca/backend
# update software
git pull
cargo update
# kill old
killall backend
# run program
nohup cargo r -r > backend_logs &