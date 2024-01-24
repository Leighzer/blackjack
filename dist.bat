cargo build --release --target x86_64-pc-windows-msvc

cp ./target/release/blackjack.exe ./dist/x86_64-pc-windows-msvc/blackjack.exe

cp ./target/release/blackjack.pdb ./dist/x86_64-pc-windows-msvc/blackjack.pdb