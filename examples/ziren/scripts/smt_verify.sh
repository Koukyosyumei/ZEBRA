mkdir report

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "z3" --num-trial 30 --ouptput-path "report/and.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "z3" --num-trial 30 --ouptput-path "report/or.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "z3" --num-trial 30 --ouptput-path "report/xor.yaml"

echo "cloclz"
./target/release/examples/cloclz --config ./configs/alu.config --opcode-str "CLO" --method "z3" --num-trial 30 --ouptput-path "report/clo.yaml"
./target/release/examples/cloclz --config ./configs/alu.config --opcode-str "CLZ" --method "z3" --num-trial 30 --ouptput-path "report/clz.yaml"

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "z3" --num-trial 30 --ouptput-path "report/mul.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULT" --method "z3" --num-trial 30 --ouptput-path "report/mult.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULTU" --method "z3" --num-trial 30 --ouptput-path "report/multu.yaml"

#echo "sll"
#./target/release/examples/shiftleft --config ./configs/alu.config --method "z3" --num-trial 30 --ouptput-path "report/sll.yaml"

echo "jump"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "Jump" --method "z3" --num-trial 30 --ouptput-path "report/jump.yaml"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "Jumpi" --method "z3" --num-trial 30 --ouptput-path "report/jumpi.yaml"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JumpDirect" --method "z3" --num-trial 30 --ouptput-path "report/jumpdirect.yaml"

echo "branch"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BEQ" --method "z3" --num-trial 30 --ouptput-path "report/beq.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BNE" --method "z3" --num-trial 30 --ouptput-path "report/bne.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BGEZ" --method "z3" --num-trial 30 --ouptput-path "report/bgez.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BGTZ" --method "z3" --num-trial 30 --ouptput-path "report/bgtz.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BLEZ" --method "z3" --num-trial 30 --ouptput-path "report/blez.yaml"
./target/release/examples/branch --config ./configs/alu.config --opcode-str "BLTZ" --method "z3" --num-trial 30 --ouptput-path "report/bltz.yaml"
