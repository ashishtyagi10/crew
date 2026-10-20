**@octo** commented on this pull request.

---

In [src/main.rs](<https://github.com/acme/app/pull/42#discussion_r1>):

```
> +    let x = parse(&input)?;
> +    run(x)
```

Should ` parse ` return a ` Result<Config> ` here? Use `` a `b` c ``.

—
Reply to this email directly, [view it on GitHub](<https://github.com/acme/app/pull/42#pullrequestreview-9>), or [unsubscribe](<https://github.com/notifications/unsubscribe-auth/X>).
You are receiving this because you were mentioned.
