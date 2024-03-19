# overriding old log file
echo "" > backend_logs

# deleted modified local cache
#
# due to issues with file ownership this must be performed
git checkout . >> backend_logs
# compile
mold --run cargo b -r >> backend_logs
# run program
nohup cargo r -r >> backend_logs &