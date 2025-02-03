### Solana Program for Belzin

**Checkout Belzin: `https://belzin.fun`**

## Instructions
- `create_bet`: Creates a new bet with specified title, bet amount and end time. Initializes a vault token account to hold funds and sets up initial state (no bets placed, not resolved).
- `place_bet`: Allows a user to place a bet in either YES or NO direction. Transfers the bet amount from user's token account to the vault and records the user's position in a new UserBet account.
- `resolve_bet`: Can only be called by bet creator after end time has passed. Sets the final outcome (true/false) and marks the bet as resolved.
- `claim_winnings`: Allows winning participants to claim their share of the total pot. Calculates winnings based on total amount and number of winners, then transfers tokens from vault to user's account. Can only be called once per winning user.
