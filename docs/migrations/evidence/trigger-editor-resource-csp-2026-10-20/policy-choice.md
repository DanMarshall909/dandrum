# Resource-origin policy choice — source-only readiness

20 October 2026. Investigation base: `9f9252cfee895f30e659e2f87a42bc833eef0e5a` in `/tmp/dandrum-lanes/runtime`. Native owner/tests were read only through `git show` at that snapshot; another agent owns live edits. This document neither approves a candidate nor starts implementation/RED. No build, test, native process, listener, dependency/cache mutation, repository/index write or Git mutation occurred. Only this report and its source manifest were written. OpenSpec discovery reports task3.2 open (7/14 tasks complete); its local-assets phrase is part of a larger native-backend/session scenario and cannot be marked complete by this slice.

## Recommendation and simpler alternative

For the stated goal of allowing the production page's required packaged resources without granting unused resource-type permissions, prefer the fixed existing response-header boundary:

```text
default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; connect-src 'self'
```

This uses the same five directives as the prior four-class proposal, replaces its unneeded local fetched-image permission with a default denial, and avoids repeating a directive for each unused class. The actual bundle requires the normal local module, stylesheet and eight fonts; React uses inline styles and inline SVG. The three notices are packaged/resolvable text, not rendered resources. Keep the independently accepted `connect-src 'self'` unchanged; local404 fetch continuation is already a positive control. No new public policy configuration or resolver route is warranted.

This recommendation intentionally chooses a default resource-type restriction as well as an origin restriction. It does **not** mean every unused facility is denied: workers can fall back to `script-src 'self'`. Do not add `worker-src 'none'` merely to complete a list. If the agreed criterion is strictly origin-only, the materially simpler, valid choice is:

```text
default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'
```

That is three directives and only two additions. The immutable 14-file resolver still bounds local URLs; it creates no external-origin allowance. Its tradeoff is permission to use existing packaged URLs in otherwise unspecified supported resource roles, for example the packaged entry in a frame, subject to engine rules/navigation policy. Those roles have no product requirement today. Removing explicit `connect-src` saves one redundant directive under this particular default, but gratuitously moves an accepted contract onto fallback; retain it in this slice.

The original `connect-src self` plus four explicit script/style/font/image rules remains the narrowest *four-row-only* change, but leaves other fetch classes without a default CSP source limit. It cannot support the broader local-only-assets statement. Neither alternative needs `data:`, `blob:`, unsafe-eval, nonce machinery, broad protocol privileges or extra MIME/route exceptions. Inline SVG is DOM content; it supplies no reason for a fetched-image permission.

The displayed inline-style allowance preserves the existing normal-page positive control's literal `style="width:37px"`; production necessity has **not** been established. React style props alone do not establish it: cached `react-dom-client.production.js:1439–1455` uses individual style properties / `setProperty`, while the staged production index has no inline style. The accepted production run had no restricting style directive. If absolute minimum permission is required, separately calibrate the unmodified production page under restrictive style rules using its actual mount/geometry; do not infer necessity from a count of `style:` tokens or from the synthetic attribute. This is an unexecuted refinement, not permission to alter the established positive contract or add unsafe-eval. The recommendations above keep that contract intact for the proposed bounded slice.

## Exact standard fallback (not native coverage)

The first present directive in each chain supplies the complete source list; values do not merge:

| Effective directive | Fallback chain after itself |
| --- | --- |
| script-src-elem / script-src-attr | script-src → default-src |
| style-src-elem / style-src-attr | style-src → default-src |
| worker-src | child-src → script-src → default-src |
| frame-src | child-src → default-src |
| connect/font/img/media/object/manifest-src | default-src |

Thus recommended script/style/font/connect permissions override default denial; image/media/object/frame/manifest fetches use it. A missing default leaves absent chains unrestricted **by CSP**, not necessarily by WebKit. Neither default governs base-uri, form-action, frame-ancestors or top-level navigation. Resource hints have special union-of-allowed-origins handling; absence of default permits them. Inline/eval outcomes require their own claims/tests. These are specification semantics, not proof of this custom-scheme engine's behavior. [CSP3 §6.1.3, §6.8.3–4](https://www.w3.org/TR/2026/WD-CSP3-20260916/#directive-default-src).

## Bounded real-page calibration and denial matrix

Keep the already specified four external rows. Before any policy change, each uses ordinary packaged `/app.js`, a fresh owned endpoint/path, no-store responses, correct MIME and independently read exact bytes. It must first succeed **inside the real WebView** with the unchanged current policy; endpoint health alone is insufficient. Font fixtures need a valid production TTF and test-endpoint-only CORS response. Preserve the accepted unmodified production Silent-preview mount, original 11 consumed payload byte/MIME/status comparisons, eight loaded family/weight sets, React inline geometry, SVG rendering, declared assets and local404 continuation before/after the eventual change. Existing production receipts are positives, not external-class baselines.

| Class | Successful unchanged browser baseline | Policy denial receipt |
| --- | --- | --- |
| Classic script | unique executed marker plus server GET | enforcing script-src-elem event (actual calibrated identity), no marker/GET |
| Stylesheet | literal computed geometry/color plus GET | enforcing style-src-elem event, unchanged control, no GET |
| Font | loaded FontFace with expected identity plus GET | enforcing font-src event, rejected load, no GET |
| Image | valid PNG load with exact natural dimensions plus GET | enforcing img-src event, error/no successful dimensions, no GET |

Install the violation listener from the ordinary local module before attempts; retain effective/violated directive, disposition, blockedURI and documentURI. Bind the engine's actual directive serialization and origin-only URL redaction; do not substitute a generic error/timeout or demand full external paths that reports legitimately strip. Aggregate attempted rows so the first failure does not hide the others. Use fresh owners/endpoints and retain source-bound RED showing successful external effects/missing enforcing denials on unchanged production. Then a header-only GREEN must show the specific enforcing receipts and no endpoint effects. Default denial may report the effective class even though the source list came from default-src; inspect, do not guess.

Additional classes are bounded characterization candidates, not mandatory forced failures:

| Class | Calibration requirement / stop rule |
| --- | --- |
| Frame | Ordinary iframe load of unique owned HTML must first produce an actual frame receipt and GET. Existing Wry navigation handler has no main-frame filter and accepts only the fixed entry; it may already prevent this. If so retain that result and do not label it new CSP RED or loosen navigation for a test. |
| Worker | Require actual supported worker startup/message and GET. A classic cross-origin constructor may fail worker origin rules without CSP. Module worker/CORS/custom-scheme support is a different baseline, not interchangeable evidence. Stop if unsupported; no blob/data exemption or fake worker route. |
| Media | Require valid decodable bytes, supported codec and actual loaded metadata/duration or sample outcome plus GET; autoplay/playback restrictions are separate. An unsupported codec/error is not a policy failure. |
| Object/embed | Plugin/PDF/HTML support and existing navigation decisions can preclude loading. Require independent rendered/executed outcome and GET before claiming a new denial; absence of plugin support is not CSP proof. |
| Manifest/hints | No production need. Any future claim needs an observable supported real-page baseline and bounded endpoint accounting; do not infer native coverage from fallback syntax. |

The installed GIR explicitly describes NavigationAction decisions for the main frame **or a subframe** (`WebKit2-4.1.gir:15753`). Pinned Wry0.57.0 `webkitgtk/mod.rs:554–583` forwards NavigationAction without frame discrimination. Classic-worker same-origin constraints are independent of CSP. [HTML worker processing model](https://html.spec.whatwg.org/multipage/workers.html#worker-processing-model).

No testshim or native-evaluated negative probes qualify this boundary. Native observer evaluation may inspect established rendering or force unused production font weights, but cannot prove page script-src enforcement. The pinned upstream WebPage source dispatches native evaluation through `executeAsynchronousUserAgentScriptInWorld`, a different execution entry from a fetched page module; the deeper ScriptController/CSP source fetch was unavailable, so this investigation does not claim a new complete engine bypass trace. [WebKitGTK2.50.4 WebPage.cpp](https://raw.githubusercontent.com/WebKit/WebKit/webkitgtk-2.50.4/Source/WebKit/WebProcess/WebPage/WebPage.cpp), lines4096–4106; [official evaluation API](https://webkitgtk.org/reference/webkit2gtk/stable/method.WebView.evaluate_javascript.html) (website documents2.42.5, not the pinned installed2.50.4).

## Separate behavior and handoff limits

Exact-entry top-level navigation remains its existing owner/test. Base URL changes and form submission are not default-src promises; characterize them separately before adding their explicit directives. Popup creation is a separate NewWindowAction/create path: pinned Wry installs a create callback only when a new-window handler exists. The navigation callback does not handle NewWindowAction. Do not infer popup denial from it or from either default.

Downloads need independent containment characterization: pinned Wry defaults download_started_handler to an accepting callback and installs its download handler accordingly; its destination code selects Downloads/current-directory and can write when a download is admitted. Neither default-src selection proves download cancellation. Preserve the separate download readiness and avoid accidentally triggering writes while calibrating resources. [WebKit policy signal](https://webkitgtk.org/reference/webkit2gtk/stable/signal.WebView.decide-policy.html) distinguishes navigation/new-window/response decisions; local pinned Wry sources determine actual registration gates.

Concurrent-source drift: snapshot-only findings must be reconciled against the native_editor agent's final owner/tests before selecting RED. I deliberately did not read/hash live native_x11.rs or its tests. Earlier readiness notes predate published production calibration and must not be used to claim bundle wiring/fonts are still unimplemented. Current accepted positives remain explicitly production React **Silent preview**, with no native session/backend acceptance. No unqualified frame/worker/media/object/manifest/hint coverage is claimed, no OpenSpec checkbox changed, and full task3.2 remains open. Source identities and unavailable upstream fetches are recorded in `editor-policy-choice-sources.json`.
