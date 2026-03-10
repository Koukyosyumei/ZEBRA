mkdir report

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "z3" --num-trial 30 --ouptput-path "report/and.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "z3" --num-trial 30 --ouptput-path "report/or.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "z3" --num-trial 30 --ouptput-path "report/xor.yaml"

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "z3" --num-trial 30 --ouptput-path "report/mul.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULH" --method "z3" --num-trial 30 --ouptput-path "report/mult.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHU" --method "z3" --num-trial 30 --ouptput-path "report/multu.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULHSU" --method "z3" --num-trial 30 --ouptput-path "report/multu.yaml"

echo "jump"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JAL" --method "z3" --num-trial 30 --ouptput-path "report/jal.yaml"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JALR" --method "z3" --num-trial 30 --ouptput-path "report/jalr.yaml"

echo "branch"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BEQ" --method "z3" --num-trial 30 --ouptput-path "report/beq.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BNE" --method "z3" --num-trial 30 --ouptput-path "report/bne.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BLT" --method "z3" --num-trial 30 --ouptput-path "report/blt.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BGE" --method "z3" --num-trial 30 --ouptput-path "report/bge.yaml"
