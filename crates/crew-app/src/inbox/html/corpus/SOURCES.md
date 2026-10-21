# Where the real-world fixtures come from

The hand-written fixtures here (`apple_reply` … `structure`) are synthetic. Five are open-source
email templates, unchanged except that their `{{placeholders}}` and template sections are filled
with sample values (`Ann`, `$42.00`, `https://example.com/…`), so they read as sent mail:

| Fixture | Source | Licence |
| --- | --- | --- |
| `marketing.html` | `cerberus-hybrid.html` from github.com/TedGoas/Cerberus | MIT, © 2017 Ted Goas |
| `notice.html` | `email.html` from github.com/leemunroe/responsive-html-email-template | MIT, © 2013 Lee Munroe |
| `pm_receipt.html` | `templates-inlined/basic/receipt` from github.com/ActiveCampaign/postmark-templates | MIT, © 2015 Wildbit |
| `pm_reset.html` | `templates-inlined/basic/password-reset`, same repository | MIT, © 2015 Wildbit |
| `pm_comment.html` | `templates-inlined/basic/comment-notification`, same repository | MIT, © 2015 Wildbit |

Each is used under the MIT License, whose notice follows.

Copyright (c) 2017 Ted Goas
Copyright (c) 2013 Lee Munroe
Copyright (c) 2015 Wildbit

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
