echo "addsub"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 1

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "bb" --num-trial 1
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "bb" --num-trial 1

echo "cpu"
./target/release/examples/cpu --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 1

echo "divrem"
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "DIV" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "DIVU" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "REM" --method "bb" --num-trial 1
./target/release/examples/divrem --config ./configs/alu.config --opcode-str "REMU" --method "bb" --num-trial 1

echo "lt"
./target/release/examples/lt --config ./configs/alu.config --opcode-str "SLT" --method "bb" --num-trial 1
./target/release/examples/lt --config ./configs/alu.config --opcode-str "SLTU" --method "bb" --num-trial 1

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULH" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHU" --method "bb" --num-trial 1
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHSU" --method "bb" --num-trial 1

echo "shiftleft"
./target/release/examples/shiftleft --config ./configs/alu.config --opcode-str "SLL" --method "bb" --num-trial 1

echo "sr"
./target/release/examples/sr --config ./configs/alu.config --opcode-str "SRL" --method "bb" --num-trial 1
./target/release/examples/sr --config ./configs/alu.config --opcode-str "SRA" --method "bb" --num-trial 1
