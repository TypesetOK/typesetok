# TYPESETOK SOURCE-AVAILABLE NON-COMMERCIAL COPYLEFT LICENSE (TOK-NCCL)
**Version 1.2 — Effective November 2026** (supersedes Version 1.1 and earlier versions for all releases accompanied by this version)

Copyright (c) 2026, The TypesetOK Project and its Original Author(s) ("Licensor").
All Rights Reserved.

---

### IMPORTANT NOTICE:
**PLEASE READ THIS DOCUMENT CAREFULLY BEFORE DOWNLOADING, ACCESSING, INSTALLING, COMPILING, OR USING THE TYPESETOK SOFTWARE. THIS IS A BINDING LEGAL AGREEMENT. BY ACCESSING, CLONING, FORKING, RUNNING, OR CONTRIBUTING TO THIS SOFTWARE, YOU AGREE TO BE BOUND BY ALL TERMS AND CONDITIONS OF THIS LICENSE. IF YOU DO NOT AGREE, DO NOT ACCESS, DOWNLOAD, COPY, RUN, MODIFY, OR USE THIS SOFTWARE.**

---

### 1. DEFINITIONS

* **"Licensor"** (or **"Original Author"**) means the original copyright owner(s) and creator(s) of the TypesetOK software project identified in the copyright notice above, and their lawful successors and assignees. No person or entity becomes a Licensor by forking, modifying, maintaining, redistributing, hosting, or contributing to the Software or any Derivative Work, including by acting as the maintainer of a fork or as a maintainer of the official repository who is not an original copyright owner. Wherever this License appears in, accompanies, or applies to a Derivative Work, the term "Licensor" continues to refer exclusively to the original Licensor of TypesetOK.
* **"Software"** means the TypesetOK computer program, including but not limited to source code, object code, headers, schemas (FlatBuffers, JSON, etc.), scripts, assets, typography algorithms, documentation, test suites, and build scripts authored by or on behalf of Licensor, excluding Third-Party Components (as defined in Section 14).
* **"Derivative Work"** means any translation, port, modification, fork, enhancement, wrapper, binding, extension, plugin, or work based upon or incorporating any portion of the Software. For the avoidance of doubt, any plugin (including plugins designed for `tok-plugin-host`), graphical user interface (GUI) shell, electron frontend, or IPC client (via `tok-ipc` or flatbuffers) whose primary purpose is to expose, wrap, execute, or integrate the core typography engine of TypesetOK constitutes a Derivative Work under this License. The foregoing examples are illustrative only and do not narrow the general definition: any work that incorporates, requires, or depends on any portion of the Software in order to provide its functionality is a Derivative Work, regardless of how small that portion is relative to the whole work and regardless of the technical means of integration, including static or dynamic linking, use as a Rust crate, npm package, or other library dependency, invocation of `tok-cli` or any other executable of the Software as a subprocess, via the command line, shell scripts, pipes, sockets, or files exchanged for processing by the Software, compilation to WebAssembly, containerization, or remote procedure calls.
* **"Non-Commercial Purpose"** means use conducted solely for personal study, private hobbyist use, non-monetized academic or scientific research, classroom teaching, non-monetized communal religious study, strictly **At-Cost** typesetting and distribution as expressly permitted under Section 3.2(a), or **Self-Authored Book** typesetting by an individual author/owner as expressly permitted under Section 3.2(b), where **NO** commercial typesetting service, typesetting fee, third-party publishing business, or unauthorized commercial exploitation is sought, obtained, or conducted.
* **"At-Cost"** means typesetting, formatting, laying out, offering, or distributing publications, documents, or typeset output solely in exchange for reimbursement of verifiable, actual, direct out-of-pocket expenses strictly incurred in producing and delivering that specific output (such as paper, printing, binding, and direct shipping or postage expenses), with **zero** profit, markup, margin, commission, administrative overhead, subscription, or compensation for typesetting labor, time, or services, and without any direct or indirect commercial advantage or financial gain.
* **"Commercial Purpose"** means ANY use, exploitation, deployment, distribution, or operation that does not strictly qualify as a Non-Commercial Purpose. Commercial Purpose includes, without limitation:
  1. Any use by, within, or for the benefit of a for-profit corporation, company, partnership, sole proprietorship, commercial publisher, printing press, typesetting studio, graphic design studio, or any commercial business enterprise (including any individual whose trade, profession, or business includes providing typesetting, layout, or pre-press services to others);
  2. Offering, hosting, running, or providing the functionality of the Software or any Derivative Work as a service, including Software-as-a-Service (SaaS), hosted platform, cloud API, remote typesetting service, or timesharing service;
  3. Charging any fee, subscription, royalty, or payment for access to, use of, execution of, or output generated by the Software (other than strictly At-Cost reimbursement expressly permitted under Section 3.2(a), or direct book sales by an eligible Self-Author under Section 3.2(b));
  4. Using the Software to typeset, format, layout, render, or produce books, periodicals, marketing materials, or publications that are intended for sale, commercial licensing, or paid distribution (other than strictly At-Cost distribution expressly permitted under Section 3.2(a), or direct sales of an individual's own Self-Authored Book expressly permitted under Section 3.2(b));
  5. Bundling, packaging, or distributing the Software or any Derivative Work alongside or inside any proprietary or paid commercial product or service;
  6. Monetizing the Software or any Derivative Work through advertisements, sponsored content, user tracking, or data collection;
  7. Producing publications or other output that are distributed nominally "free of charge" but are financed, in whole or in part, by sponsorships, advertisements, patron or donor listings, "suggested" or "requested" donations, fundraising campaigns, or any other payment or consideration linked to the specific publication or its distribution (other than strictly At-Cost reimbursement expressly permitted under Section 3.2). For the avoidance of doubt, memorial dedications (*le'ilui nishmat*) printed in a publication that is distributed entirely free of charge or At-Cost do not, by themselves, constitute a Commercial Purpose, even if made in connection with a contribution toward the cost of the publication.
* **"Contribution"** means any computer code, pull request, patch, bug fix, feature, script, schema, documentation, translation, graphic, idea, or enhancement submitted, conveyed, or committed by any person or entity to the Licensor, the Software repository, or the project’s communication channels. For purposes of Section 6, "Contribution" also includes any modification, addition, patch, feature, bug fix, or code authored, committed, or incorporated in any fork or Derivative Work of the Software.
* **"Contributor"** means any individual, corporation, or entity that submits a Contribution.
* **"You"** (or **"Your"**) means the individual or legal entity exercising any right under this License or accessing the Software, together with all of its Affiliates. **"Affiliate"** means any entity that directly or indirectly controls, is controlled by, or is under common control with You. Use of the Software by an individual in the course of, or in connection with, employment or engagement by any entity, or for the benefit of any entity, is deemed use by that entity. Routing any use of the Software, any Derivative Work, or any output through an Affiliate, non-profit vehicle, nominee, employee, contractor, or other intermediary in order to obtain a benefit that would constitute a Commercial Purpose if obtained directly is itself a Commercial Purpose.
* **"Source Code"** means the preferred form of the work for making modifications to it, in full human-readable format with complete build and configuration scripts.

---

### 2. GRANT OF RIGHTS (NON-COMMERCIAL USE ONLY)

Subject to the express conditions set forth in Section 3 and throughout this License, Licensor hereby grants you a personal, worldwide, non-exclusive, royalty-free, revocable license to:
1. Download, compile, execute, and study the Software solely for **Non-Commercial Purposes**;
2. Create and run Derivative Works of the Software solely for **Non-Commercial Purposes**;
3. Distribute verbatim or modified copies of the Software solely for **Non-Commercial Purposes**, strictly subject to the Copyleft requirements in Section 4.

**EXPRESS CONDITION OF GRANT (JACOBSEN V. KATZER DOCTRINE):**
Every restriction, requirement, and condition set forth in this License (including, without limitation, the Non-Commercial restriction in Section 3, the Copyleft obligations in Section 4, the Network Use clause in Section 5, and the Contributor terms in Section 6) constitutes an **express condition precedent** to the grant of rights hereunder, and not merely a contractual covenant. Any use, copying, modification, execution, or distribution of the Software outside the explicit scope of these conditions terminates this License immediately and automatically, and constitutes **willful copyright infringement** giving rise to statutory damages, injunctive relief, and all remedies available under international copyright law.

---

### 3. STRICT PROHIBITION OF COMMERCIAL USE

1. **Absolute Commercial Ban:** You may NOT use, execute, run, modify, fork, integrate, or distribute the Software or any Derivative Work for any **Commercial Purpose**, whether directly or indirectly, without obtaining a separate, valid, written commercial license executed directly by the Licensor.
2. **Commercial Output Clause & Permitted Exceptions (At-Cost and Self-Authored Books):** Using the Software to typeset, format, or layout books, documents, or publications that are sold, leased, licensed, or otherwise distributed in exchange for any payment or consideration constitutes a Commercial Purpose and requires a Commercial License from the Licensor, **except strictly as provided in subsections (a) and (b) below**:
   - **(a) At-Cost Typesetting & Distribution Exception:** Any person or entity who typesets, formats, or lays out books, documents, or publications using the Software and offers or distributes the resulting typeset output or publications **strictly At-Cost** (as defined in Section 1 — recovering only verifiable, actual, direct out-of-pocket expenses such as printing, paper, binding, and direct shipping/postage, with **zero** profit, markup, margin, administrative overhead, or compensation for typesetting labor or services) is permitted to do so under Non-Commercial Use, provided that:
     1. No profit, markup, commission, or indirect commercial advantage is sought, charged, or obtained;
     2. You do not split, bundle, or disguise any profit, markup, or typesetting labor fee as a separate "handling", "consulting", "editing", "design", "administration", inflated shipping, or "suggested donation" charge;
     3. The At-Cost offering or distribution is not conducted to promote, subsidize, bundle with, or drive traffic to any paid or commercial product, service, or business enterprise; and
     4. Upon Licensor's reasonable request, the party relying on this At-Cost exception can document and substantiate the actual direct out-of-pocket costs.
   - **(b) Self-Authored Book Exception (Individual Author/Owner):** An individual natural person who is the genuine original author, compiler, or primary literary copyright owner of a book or scholarly/religious work (**"Self-Author"**) may personally use the Software to typeset that specific work and may sell, distribute, or monetize copies of that finished book without obtaining a Commercial License, **STRICTLY SUBJECT TO ALL OF THE FOLLOWING ANTI-CIRCUMVENTION CONDITIONS**:
     1. **Natural Person & Non-Typesetter Status:** This exception applies **only** to an individual natural person acting on their own behalf. It does **NOT** apply to any commercial publishing house, for-profit corporation, partnership, printing press, typesetting studio, graphic design agency, or any person or entity whose trade, profession, business, or paid service includes typesetting, layout, or pre-press work for others.
     2. **Personal Operation Only (No Typesetting-for-Hire):** The Self-Author must **personally** operate the Software to typeset their own work. Hiring, engaging, or compensating a third-party typesetter or studio to use the Software on the Self-Author's behalf is strictly prohibited unless that third-party typesetter holds a valid Commercial License from the Licensor.
     3. **No Ghost-Ownership, Nominee, or Sham Assignment Loophole:** You may NOT circumvent this License by transferring, assigning, or licensing nominal copyright, co-authorship, "publisher" status, or distribution rights in a third party's manuscript to a typesetter, studio, or intermediary (or vice versa), nor by using a shell entity, imprint, or nominal "self-publishing" arrangement to typeset, format, or publish works authored primarily by other people for sale or fee.
     4. **No Separate Fee for Typesetting:** This exception permits selling the Self-Author's finished book; it **never** permits charging any fee, service charge, or separate consideration for the act of typesetting, formatting, or using the Software.
   - **(c) Third-Party Physical Print Shops Permitted:** For the avoidance of doubt, sending a finished, static print-ready file (such as a PDF) produced under valid Non-Commercial, At-Cost (Section 3.2(a)), or Self-Authored Book (Section 3.2(b)) use to a commercial printing press or bindery **solely for physical printing and binding** is permitted and does not violate this License, provided that the commercial printer does not access, execute, or run the Software itself.
   - **(d) Everything Else Requires a Commercial License:** Any pricing exceeding actual direct out-of-pocket costs (outside Section 3.2(b)), any fee or remuneration for typesetting labor or services, or any commercial publishing or distribution by a publisher, studio, or business entity remains a Commercial Purpose requiring a written Commercial License from the Licensor, regardless of whether the seller or distributor is a for-profit or non-profit entity. Personal, communal, non-monetized scholarly or religious publications distributed entirely free of charge, and not financed by any of the means described in item 7 of the definition of Commercial Purpose, are likewise permitted under Non-Commercial Use.
3. **No Laundering of Output:** The Commercial Output Clause attaches to the output itself, irrespective of Your intention at the time the output was created. If any document, PDF, print file, or publication produced with the Software or any Derivative Work is subsequently sold, licensed, transferred for consideration, or otherwise used for any Commercial Purpose — whether by You or by any third party with Your authorization or assistance — such use requires a Commercial License from the Licensor. Producing output for, or delivering output to, a third party for use in a Commercial Purpose constitutes a Commercial Purpose.
4. **Machine Learning & Automated Reimplementation:** You may not use the Software, any Derivative Work, or any portion thereof as training, fine-tuning, evaluation, embedding, or retrieval data for any artificial intelligence or machine learning model or system, except for a Non-Commercial Purpose in which neither the resulting model nor its outputs are distributed, offered as a service, or used for any Commercial Purpose. Any translation, port, rewrite, or reimplementation of the Software or of any part of it that is produced by, or with the assistance of, an automated tool (including AI code assistants and large language models) that was given access to the Software's Source Code is a Derivative Work, and is subject to this License in full.
5. **Licensor Exclusivity & Commercial Reservation:** The Licensor explicitly reserves all rights to commercialize, monetize, license, dual-license, or sell the Software or any part thereof. The Non-Commercial restriction in this License applies exclusively to third-party licensees and Contributors, and in no way limits or restricts the Licensor from commercializing the Software under any terms Licensor chooses.

---

### 4. MANDATORY COPYLEFT & OPEN SOURCE REQUIREMENTS FOR FORKS (SHARE-ALIKE)

If you modify, fork, adapt, wrap, or create a Derivative Work of the Software, or if you distribute the Software or any Derivative Work to any third party (or deploy it over a network pursuant to Section 5), you must comply with all of the following requirements:
1. **Identical License (Share-Alike):** The entire fork, modification, or Derivative Work must be licensed exclusively under the **exact terms of this TOK-NCCL License**. You may not alter, remove, add, or waive any term, restriction, or condition of this License. You may not relicense or dual-license the Software or any Derivative Work under MIT, Apache, BSD, GPL, AGPL, Creative Commons, or any other license.
2. **Mandatory Full Source Code Availability:** You must make the complete, machine-readable Source Code of the entire fork or Derivative Work publicly and freely available, under this exact License, without requiring any fee, registration, login wall, or nondisclosure agreement.
3. **Preservation of Notices:** You must retain all copyright notices, author credits, and copies of this License in all source files, documentation, and user interfaces.
4. **Anti-Circumvention & Anti-Locking (Anti-Tivoization):** You may not use digital rights management (DRM), hardware encryption, obfuscation, or technical protection measures that prevent or restrict any user from inspecting, modifying, or recompiling your Derivative Work.
5. **Clear Notice of Changes:** You must cause modified files to carry prominent notices stating that you changed the files and the date of any change.
6. **Automatic Upstream Grant to Main Project (Fork License-Back):** By the very act of creating, hosting, publishing, or maintaining a fork or Derivative Work of the Software (whether on a public code-hosting platform such as GitHub or GitLab, or privately, and regardless of whether a pull request or formal submission is ever made), You automatically and irrevocably grant to the Licensor (for the main TypesetOK project and any Licensor-authorized editions) all rights, licenses, waivers, warranties, and patent grants set forth in Section 6 with respect to all modifications, additions, patches, features, and code authored, committed, or incorporated in Your fork or Derivative Work. The Licensor may freely use, incorporate, port, modify, distribute, relicense, and commercialize any such code in the main project without further consent, royalty, or obligation.

---

### 5. NETWORK ACCESS & CLOUD DEPLOYMENT (CLOSING THE SAAS LOOPHOLE)

1. **Network Interaction as Distribution:** Making the functionality of the Software or any Derivative Work accessible to any person or entity over a computer network (including the Internet, private intranet, VPN, web interface, cloud service, container, or REST/RPC API) constitutes distribution of the Software under this License.
2. **Prohibition of Commercial SaaS:** Deploying, hosting, or providing the Software or any Derivative Work as a cloud service, SaaS, hosted typesetting engine, or paid API for any commercial purpose or monetary gain is strictly forbidden without a commercial license from the Licensor.
3. **Non-Commercial Network Obligations:** If you deploy or make the Software or any Derivative Work accessible over a network for a Non-Commercial Purpose, you must provide a prominent, publicly accessible hyperlink on the user interface or API documentation directing users to download the complete corresponding Source Code of your deployment under this License.

---

### 6. INBOUND CONTRIBUTOR & FORK LICENSE AGREEMENT (CLA) & RIGHTS ASSIGNMENT
*(CLOSING THE CONTRIBUTOR & FORK LOCK-IN / HOLD-UP LOOPHOLE)*

By creating, hosting, publishing, or maintaining a fork or Derivative Work of the Software, or by submitting, communicating, opening a pull request, or committing any **Contribution** to the Licensor, the Software repository, or the project:

1. **Broad, Irrevocable Grant of Rights to Licensor:** You hereby grant to the Licensor an unconditional, irrevocable, perpetual, worldwide, non-exclusive, royalty-free, fully paid-up, assignable, and fully sublicensable license to:
   - Use, reproduce, modify, adapt, prepare derivative works of, display, perform, compile, execute, distribute, and publish your Contribution (and any derivative works thereof);
   - Dual-license, relicense, sell, sub-license, and commercialize your Contribution under any terms Licensor chooses, including under proprietary, closed-source, commercial, or alternative open-source licenses, with or without charging fees;
   - Exploit, patent, or monetize the Contribution in any medium or technology now known or hereafter devised.
2. **No Rights, Royalties, or Leverage Conferred:** You explicitly acknowledge, agree, and stipulate that:
   - Your Contribution is made voluntarily, and you will receive **NO financial compensation, royalty, equity, ownership share, or property interest** in the project or Software;
   - Submitting a Contribution does **NOT** make you a co-owner, partner, joint author, or member of the Licensor;
   - You have **NO veto power, no blocking rights, and no standing** to challenge, prevent, or restrict the Licensor from modifying the License, relicensing the project, offering commercial editions, or entering into commercial contracts;
   - Licensor owes you **NO ongoing duty, accounting, fiduciary duty, or obligation of any kind**.
3. **Waiver of Moral Rights:** To the fullest extent permitted by applicable law, you hereby irrevocably waive, release, and covenant not to assert any and all "moral rights" (*droit moral*), rights of paternity, attribution, integrity, disclosure, or withdrawal with respect to your Contribution as against the Licensor, Licensor's successors, assignees, and licensees.
4. **Contributor Warranties:** You represent and warrant that:
   - You are the sole and original author of the Contribution, or you have full legal right and authorization from your employer or copyright owner to grant the rights set forth herein;
   - Your Contribution does not infringe or misappropriate any third-party copyright, patent, trademark, trade secret, or other proprietary right;
   - Your Contribution is not subject to any third-party license or encumbrance (such as GPL, AGPL, or proprietary employer IP) that would conflict with this License.
5. **Contributor Patent License:** You hereby grant to the Licensor, its successors, assignees, and commercial licensees, and to every recipient of the Software to the extent of the rights such recipient holds under this License, a perpetual, worldwide, non-exclusive, royalty-free, fully paid-up, irrevocable patent license to make, have made, use, sell, offer for sale, import, and otherwise transfer Your Contribution and the Software as combined with Your Contribution, under any patent claims now or hereafter owned or licensable by You or Your Affiliates that are infringed by Your Contribution alone or by its combination with the Software. You covenant not to assert, and to cause Your Affiliates not to assert, any such patent claim against the Licensor or any such recipient with respect to such activities.

---

### 7. PATENT RETALIATION CLAUSE

If you, your affiliate, or any entity under your control institutes, files, initiates, or participates in any patent infringement litigation, arbitration, or proceeding against the Licensor or any user of the Software (including cross-claims or counterclaims), alleging that the Software or any Contribution constitutes or contributes to patent infringement, then all licenses and permissions granted to you under this License shall **terminate immediately and retroactively** as of the date you first received, cloned, or used the Software.

---

### 8. TRADEMARK & IDENTITY PROTECTION

This License does not grant any right or license to use the trademarks, trade names, service marks, logos, or domain names of the Licensor, including but not limited to the names **"TypesetOK"**, **"TOK"**, or any associated logos. Any forked, modified, or derivative version of the Software must be clearly and distinctly renamed, and may not use the name "TypesetOK" or "TOK" in its title, brand, or package name without prior express written permission from the Licensor. You may not use Licensor's name to endorse or promote any product derived from the Software.

The notice-preservation obligations of Section 4.3 and the restrictions of this Section 8 apply cumulatively and do not conflict: You must retain all existing copyright notices and author credits, and must state truthfully that Your work is derived from TypesetOK (for example, "Based on TypesetOK"), but such limited, factual attribution grants no right to use "TypesetOK" or "TOK" as, or as part of, a product name, brand, logo, domain name, or published package name, or in any manner suggesting endorsement by or affiliation with the Licensor. Internal source-code identifiers inherited from the Software (such as crate, module, or package names beginning with `tok-`) may be retained in Source Code solely for technical compatibility, but may not be published to any public package registry, or used as a public-facing product name, without the Licensor's prior written permission. Compliance with Section 4.3 is never a defense to a violation of this Section 8, and vice versa.

---

### 9. ABSOLUTE DISCLAIMER OF WARRANTIES
*(PROTECTION AGAINST SOFTWARE & PRINT DEFECT CLAIMS)*

1. **"AS IS" BASIS:** THE SOFTWARE IS PROVIDED BY THE LICENSOR "AS IS" AND "AS AVAILABLE", WITH ALL FAULTS, DEFECTS, AND ERRORS, WITHOUT WARRANTY OF ANY KIND, WHETHER EXPRESS, IMPLIED, STATUTORY, OR OTHERWISE.
2. **FULL DISCLAIMER:** TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, THE LICENSOR SPECIFICALLY AND EXPRESSLY DISCLAIMS ALL WARRANTIES, INCLUDING BUT NOT LIMITED TO:
   - THE IMPLIED WARRANTIES OF MERCHANTABILITY, SATISFACTORY QUALITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE, ACCURACY, QUIET ENJOYMENT, AND NON-INFRINGEMENT;
   - ANY WARRANTIES ARISING OUT OF COURSE OF DEALING, USAGE, OR TRADE PRACTICE;
   - ANY WARRANTY THAT THE SOFTWARE WILL BE UNINTERRUPTED, BUG-FREE, SECURE, ACCURATE, OR COMPATIBLE WITH ANY HARDWARE, OPERATING SYSTEM, PRINTER, OR FONT;
   - ANY WARRANTY REGARDING TYPOGRAPHY, PRE-PRESS OUTPUT, PDF CONVERSION, PDF/X COMPLIANCE, FONT SHAPING, BIDI/HEBREW RENDERING, COLOR SEPARATION, CMYK TRAPPING, BLEED MARKS, MARGIN CALCULATIONS, HYPHENATION, OR PRINT-READINESS.
3. **USER ASSUMES ALL RISK:** YOU EXPRESSLY ACKNOWLEDGE AND AGREE THAT THE ENTIRE RISK AS TO THE RESULTS, ACCURACY, QUALITY, SUITABILITY, AND PERFORMANCE OF THE SOFTWARE (INCLUDING ANY DOCUMENTS, BOOKS, OR PRE-PRESS FILES GENERATED BY THE SOFTWARE) IS ENTIRELY WITH YOU.

---

### 10. STRICT LIMITATION OF LIABILITY
*(SHIELDING FROM DAMAGES, PRINT ERRORS & FINANCIAL LOSS)*

TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW:
1. **NO LIABILITY FOR ANY DAMAGES:** UNDER NO CIRCUMSTANCES AND UNDER NO LEGAL THEORY (WHETHER IN CONTRACT, TORT, NEGLIGENCE, STRICT LIABILITY, PRODUCT LIABILITY, OR OTHERWISE) SHALL THE LICENSOR, ITS AUTHORS, CONTRIBUTORS, OR MAINTAINERS BE LIABLE TO YOU OR ANY THIRD PARTY FOR ANY DIRECT, INDIRECT, SPECIAL, INCIDENTAL, PUNITIVE, EXEMPLARY, OR CONSEQUENTIAL DAMAGES OF ANY KIND, ARISING OUT OF OR IN CONNECTION WITH THIS LICENSE OR THE USE, INABILITY TO USE, MODIFICATION, OR DISTRIBUTION OF THE SOFTWARE.
2. **SPECIFIC PRINT & BUSINESS EXCLUSIONS:** WITHOUT LIMITING THE FOREGOING, LICENSOR SHALL HAVE ZERO LIABILITY FOR:
   - PRINTING DEFECTS, DEFECTIVE BOOK RUNS, MISPRINTED EDITIONS, WASTED PAPER, INK, OR PRINTING PLATES;
   - PRINTER OR BINDERY INVOICES, CANCELLATION CHARGES, OR EXPENSES INCURRED IN REPRINTING RUNS;
   - DELAYED BOOK LAUNCHES, MISSED PUBLICATION DEADLINES, PENALTIES UNDER PUBLISHING CONTRACTS, OR LOSS OF CLIENT CONTRACTS;
   - LOSS OF PROFITS, LOSS OF REVENUE, LOSS OF GOODWILL, LOSS OF DATA, CORRUPTION OF TYPESETTING PROJECTS, OR BUSINESS INTERRUPTION;
   - EQUIPMENT FAILURE, DATA LOSS, OR SECURITY BREACHES.
3. **CUMULATIVE CAP:** IN NO EVENT SHALL LICENSOR'S TOTAL CUMULATIVE LIABILITY ARISING OUT OF OR RELATING TO THIS LICENSE OR THE SOFTWARE EXCEED THE TOTAL AMOUNT ACTUALLY PAID BY YOU TO LICENSOR (IF ANY) UNDER THIS SPECIFIC LICENSE, WHICH SHALL BE ZERO DOLLARS ($0.00 USD).
4. **ESSENTIAL BASIS OF BARGAIN:** THE PARTIES AGREE THAT THE DISCLAIMERS OF WARRANTY AND LIMITATIONS OF LIABILITY IN SECTIONS 9 AND 10 REFLECT A DELIBERATE ALLOCATION OF RISK AND FORM AN ESSENTIAL BASIS OF THE BARGAIN UPON WHICH THE SOFTWARE IS PROVIDED FREE OF CHARGE.

---

### 11. INDEMNIFICATION

You agree to defend, indemnify, and hold harmless the Licensor, original authors, and maintainers from and against any and all claims, demands, lawsuits, proceedings, liabilities, losses, damages, settlements, judgments, costs, and expenses (including reasonable attorney's fees) arising out of or resulting from:
1. Your use, execution, modification, or distribution of the Software or any Derivative Work;
2. Any publication, printed book, PDF, or commercial product created, formatted, or produced using the Software;
3. Any breach by you of any term, condition, or warranty of this License;
4. Any claim that any Contribution or modification made by you infringes or misappropriates any third-party intellectual property or privacy right.

---

### 12. AUTOMATIC TERMINATION & REMEDIES

1. **Automatic Termination:** Any breach of any condition, obligation, or restriction set forth in this License shall cause this License and all rights granted to you to terminate **automatically and immediately**, without the need for notice or legal action by Licensor.
2. **Effect of Termination:** Upon termination, you must immediately cease all access, execution, copying, modification, and distribution of the Software and permanently destroy all copies in your possession. Any continued use constitutes willful copyright infringement.
3. **Injunctive Relief:** You acknowledge that unauthorized commercial use or violation of this License will cause irreparable harm to Licensor for which monetary damages alone would be inadequate. Accordingly, Licensor shall be entitled to seek immediate preliminary and permanent injunctive relief, specific performance, and statutory damages in any court of competent jurisdiction, without posting bond or proving actual financial damages.
4. **Attorney's Fees:** In the event Licensor enforces this License against you in any legal proceeding and prevails, Licensor shall be entitled to recover from you all reasonable attorney's fees, expert costs, and expenses incurred in enforcement.
5. **No Automatic Reinstatement:** A terminated license may be reinstated only by an express written notice signed by the Licensor. Re-downloading, re-cloning, re-installing, or otherwise re-obtaining the Software (including any later release, or a copy received from a third party) does not grant You a new license or reinstate a terminated one.
6. **Survival:** Termination does not affect (a) the rights granted to the Licensor and to recipients under Section 4.6 and Section 6, which are irrevocable; (b) Sections 1, 3.5, 7, 8, 9, 10, 11, 12 and 13, which survive any termination or expiration of this License; or (c) Your liability for any breach occurring before termination.

---

### 13. MISCELLANEOUS & SEVERABILITY

1. **Severability:** If any provision of this License is determined by a court of competent jurisdiction to be invalid, illegal, or unenforceable, such provision shall be enforced to the maximum extent permissible, and all other provisions shall continue in full legal force and effect.
2. **Governing Law:** This License shall be governed by, construed, and enforced in accordance with the laws of the State of Israel (or, at Licensor's election, the jurisdiction where Licensor resides or where infringement occurs), without giving effect to conflict-of-law principles. The United Nations Convention on Contracts for the International Sale of Goods (CISG) shall not apply.
3. **Entire Agreement; Written Permissions Only:** This License constitutes the complete and exclusive agreement between the parties regarding the subject matter hereof and supersedes all prior proposals and understandings, whether written or oral, **except that it does not supersede, amend, or limit any separate written commercial license or other written agreement executed directly by the Licensor, which shall prevail to the extent of any conflict.** No permission, exception, waiver, or commercial authorization is valid unless it is made in a writing signed by the Licensor. Statements, comments, issue or pull-request replies, chat or forum messages, or conduct of any Contributor, maintainer, or community member do not grant any right and do not bind the Licensor.
4. **No Waiver:** No delay or failure by Licensor to exercise any right or power under this License shall operate as a waiver thereof.
5. **Reservation of Rights; No Implied License:** All rights not expressly granted in this License are reserved by the Licensor. No license or right is granted by implication, estoppel, exhaustion, course of dealing, the public availability of the Source Code, the Licensor's tolerance of any use, or otherwise.

---

### 14. THIRD-PARTY COMPONENTS

1. **Definition and Precedence:** "Third-Party Components" means fonts, libraries, and other materials owned by persons other than the Licensor that are distributed with or used by the Software, including fonts licensed under the SIL Open Font License located under `assets/fonts/` (each accompanied by its `OFL-*.txt` license file) and open-source libraries obtained through Cargo, npm, or bundled runtimes such as Electron. Third-Party Components are not licensed by the Licensor under this License and remain governed exclusively by their own license terms, which prevail over this License with respect to those components only.
2. **No Effect on the Software:** Section 14.1 does not permit any use of the Software or any Derivative Work outside the terms of this License. Separating, extracting, replacing, or relicensing a Third-Party Component confers no right in the Software, and the presence of a permissively licensed Third-Party Component in a combined work does not make any portion of the Software available under that permissive license.

---

### 15. LICENSE VERSIONS

1. **Versions:** The Licensor may publish revised versions of this License, each identified by a distinct version number. Each release of the Software is governed by the version of this License that accompanies that release.
2. **No Cherry-Picking:** Any Derivative Work that incorporates any portion of a release of the Software governed by a later version of this License is governed, as a whole, by that later version. You may not combine portions of releases governed by different versions in order to rely on the terms of an earlier version.
3. **Share-Alike Reference:** References in Section 4 to the "exact terms of this TOK-NCCL License" mean the version of this License that governs the Derivative Work under this Section 15.
