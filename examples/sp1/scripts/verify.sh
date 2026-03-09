echo "addsub"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 1

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "bb" --num-trial 1

echo "jump"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JAL" --method "bb" --num-trial 1
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JALR" --method "bb" --num-trial 1

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULH" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHU" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHSU" --method "bb" --num-trial 1

echo "sll"
./target/release/examples/shiftleft --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1