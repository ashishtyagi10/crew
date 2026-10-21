# Where the real-world fixtures come from

The hand-written fixtures here (`apple_reply` … `structure`) are synthetic. The rest are
open-source email templates, changed only so that they read as sent mail:

- `{{placeholders}}` and template sections are filled with sample values (`Ann`, `$42.00`,
  `https://example.com/…`);
- `href="#"` is replaced with an example address;
- relative picture paths are replaced with absolute `https://cdn.example.com/…` ones.

| Fixture | Source | Licence |
| --- | --- | --- |
| `marketing.html` | `cerberus-hybrid.html` from github.com/TedGoas/Cerberus | MIT, © 2017 Ted Goas |
| `notice.html` | `email.html` from github.com/leemunroe/responsive-html-email-template | MIT, © 2013 Lee Munroe |
| `pm_receipt.html` | `templates-inlined/basic/receipt` from github.com/ActiveCampaign/postmark-templates | MIT, © 2015 Wildbit |
| `pm_reset.html` | `templates-inlined/basic/password-reset`, same repository | MIT, © 2015 Wildbit |
| `pm_comment.html` | `templates-inlined/basic/comment-notification`, same repository | MIT, © 2015 Wildbit |
| `cl_shop.html` | `6/index.html` (Shopping) from github.com/ColorlibHQ/email-templates | MIT, © Colorlib |
| `cl_cart.html` | `12/index.html` (Shop, cart reminder), same repository | MIT, © Colorlib |
| `cl_order.html` | `22/index.html` (Northbound, order receipt), same repository | MIT, © Colorlib |
| `cl_brief.html` | `24/index.html` (Brief, editorial newsletter), same repository | MIT, © Colorlib |
| `cl_launch.html` | `25/index.html` (Loop, product launch), same repository | MIT, © Colorlib |
| `cl_dark.html` | `28/index.html` (Pulse, dark-mode newsletter), same repository | MIT, © Colorlib |

Each is used under the MIT License, whose notice follows.

Copyright (c) 2017 Ted Goas
Copyright (c) 2013 Lee Munroe
Copyright (c) 2015 Wildbit
Copyright (c) Colorlib

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
