mkdir report

./target/release/examples/add32 --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 5 --ouptput-path "report/add32.yaml"
./target/release/examples/sub32 --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 5 --ouptput-path "report/sub32.yaml"

#num files: 11
#average exe_time_mean: 0.00227
