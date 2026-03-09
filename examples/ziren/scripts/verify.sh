mkdir report

echo "addsub"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "ADD" --method "bb" --num-trial 30 --ouptput-path "report/add.yaml"
./target/release/examples/addsub --config ./configs/alu.config --opcode-str "SUB" --method "bb" --num-trial 30 --ouptput-path "report/sub.yaml"

echo "bitwise"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "AND" --method "bb" --num-trial 30 --ouptput-path "report/and.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "OR" --method "bb" --num-trial 30 --ouptput-path "report/or.yaml"
./target/release/examples/bitwise --config ./configs/alu.config --opcode-str "XOR" --method "bb" --num-trial 30 --ouptput-path "report/xor.yaml"

echo "cloclz"
./target/release/examples/cloclz --config ./configs/alu.config --opcode-str "CLO" --method "bb" --num-trial 30 --ouptput-path "report/clo.yaml"
./target/release/examples/cloclz --config ./configs/alu.config --opcode-str "CLZ" --method "bb" --num-trial 30 --ouptput-path "report/clz.yaml"

echo "movcond"
./target/release/examples/movcond --config ./configs/alu.config --opcode-str "MEQ" --method "bb" --num-trial 30 --ouptput-path "report/meq.yaml"
./target/release/examples/movcond --config ./configs/alu.config --opcode-str "MNE" --method "bb" --num-trial 30 --ouptput-path "report/mne.yaml"
./target/release/examples/movcond --config ./configs/alu.config --opcode-str "WSBH" --method "bb" --num-trial 30 --ouptput-path "report/wsbh.yaml"

echo "mul"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MUL" --method "bb" --num-trial 30 --ouptput-path "report/mul.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULT" --method "bb" --num-trial 30 --ouptput-path "report/mult.yaml"
./target/release/examples/mul --config ./configs/alu.config --opcode-str "MULTU" --method "bb" --num-trial 30 --ouptput-path "report/multu.yaml"

echo "sll"
./target/release/examples/shiftleft --config ./configs/alu.config --method "bb" --num-trial 30 --ouptput-path "report/sll.yaml"

echo "jump"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "Jump" --method "bb" --num-trial 30 --ouptput-path "report/jump.yaml"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "Jumpi" --method "bb" --num-trial 30 --ouptput-path "report/jumpi.yaml"
./target/release/examples/jump --config ./configs/alu.config --opcode-str "JumpDirect" --method "bb" --num-trial 30 --ouptput-path "report/jumpdirect.yaml"