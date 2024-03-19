# overriding old log file
echo "" > backend_logs
# run program
nohup cargo r -r >> backend_logs &