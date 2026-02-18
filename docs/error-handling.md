# Error handling

Knox encourages explicit error handling using **`Result<T, E>`** and the **`?`** operator. There is no global try/catch; errors are values.

## Result type

A value of type `Result<T, E>` is either:

- **`Ok(value)`** — success, carrying a value of type `T`.
- **`Err(error)`** — failure, carrying a value of type `E` (often an error type or string).

Example:

```kx
fn parseNumber(s: string) -> Result<int, string> {
  // ... attempt parse ...
  Ok(42)
  // or on failure:
  // Err("invalid number")
}
```

## The `?` operator

Inside a function that returns `Result<T, E>`, you can use `?` on an expression of type `Result<A, E>`:

- If it’s **`Ok(x)`**, the expression evaluates to `x` and execution continues.
- If it’s **`Err(e)`**, the function returns `Err(e)` immediately (early return).

So `?` propagates errors up the call stack without writing `match` every time.

Example:

```kx
fn transfer(sender: Account, to: Address, amount: u64) -> Result<(), Error> {
  let bal = sender.balance()
  if bal < amount { return Err(Error::InsufficientFunds) }
  sender.debit(amount)?
  to.credit(amount)?
  Ok(())
}
```

Here, if `debit` or `credit` returns `Err`, the function returns that error; otherwise execution continues and finally returns `Ok(())`.

## No null for errors

Knox does not use `null` or sentinel values for errors. Success or failure is always encoded in the type (`Result`), so the compiler forces you to handle both cases (or explicitly propagate with `?`). That keeps error handling visible and predictable.
