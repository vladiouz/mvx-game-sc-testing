#### `deploy()` endpoint

strange scenario: when `game_start_fee_opt` is not provided, the error message is **fee token id not set** instead of **game start fee not set**
same is happening when `enabled_opt` is `OptionalValue::None`

#### `create_game()` endpoint

?? a nice to have feature would be to let the user specify a certain time limit instead of just `waiting_time`

#### `join_game()` endpoint

```
require!(
            !self.game_settings(game_id).is_empty(),
            "no settings for game id"
        );
        let game_settings = self.game_settings(game_id).get();
```

## Qs

Are we supposed to minimize overlapping during integration testing?

How do we assert that a specific amount has been sent to an user? (Other than using the API)
