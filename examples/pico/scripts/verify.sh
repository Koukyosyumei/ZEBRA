mkdir report

echo "addsub"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 5 --ouptput-path "report/add.yaml"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 5 --ouptput-path "report/sub.yaml"

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "bb" --num-trial 5 --ouptput-path "report/and.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "bb" --num-trial 5 --ouptput-path "report/or.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "bb" --num-trial 5 --ouptput-path "report/xor.yaml"

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "bb" --num-trial 5 --ouptput-path "report/mul.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULH" --method "bb" --num-trial 5 --ouptput-path "report/mult.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHU" --method "bb" --num-trial 5 --ouptput-path "report/multu.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHSU" --method "bb" --num-trial 5 --ouptput-path "report/multu.yaml"

echo "sll"
./target/release/examples/sll --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 5 --ouptput-path "report/sll.yaml"

# average exe_time_mean: 0.07285856208888888
