
## intro
I got adhd from hell, so I use the terminal all the time. 
I could not find a good terminal project management tool so I built my own and named it NerdTime.

The time tracker needed an:

- **CLI** (`nerd start/stop/status/log/sync/login`) 
- **Backend API**

I build it in 2 hours with **OpenCode**, **DeepSeek v4 flash** and **Weaponized Autism**.

Because of the rapid build, I got inspired and whent for my streech goals of:

- **Insights**
- **Heatmap**
- **Devlog**
- **MCP** 

So over about 10 hours I build a very simple **microSaaS**, I never deployed it but is is functionally done.

## what I learned* 

**AI is a powerfull tool but no repalcment for skill**.

I build this is a Hybrid of Spec Driven Deven development **(SDD)** and VibeCoding. 

I did this for 2 reasons:

- Rapid prototyping
- to see if professional high quality software con be built with AI alone

One can rapid prototype a app with SDD an VibeCoding, but I did not get professional code quality though. This may be a skill issue.


### My Process

I had a Idea for a MicroSaaS and I already had the domain but never had the time to build it.

So I chatted with Gemini and did some market research, after defining the product I defined the features, commands, architecture etc in a back and forth conversation with Gemini. after I was thought I thought of everything(FORSHOADOING)I downloeaded the PDF and opened it up in OpenCode

### what went well

After about 2 hours of doing this I had a working app, I actually started to use it and I started adding my streach goals. I felt like a coding god even though I did not touch or right a single line of code. 

### why it went well

I define a somewhat specific and professional spec. Based on my conversation I had with the AI I was able to git it to write a architecture and pick a stack. Gemini also picked a decent language and business model.

### how can I improve

I have ideas on how I can improve this. One thing I need to use use a better model, I used Gemini Flash not Pro. Another Think I can do to improve my workflow is to not delegate the architecture of the app to the AI but to augment my thinking but this was a experiment to see if AI can do everything so to that end maybe feed the AI templates and stick to one framework and Design Pattern.

The More you specify the AI the less of a change it has at holisating and writing buggy code. So you need to make each spec as airthight as possible and as autisticly detailed as possible 

I also recommend sticking to rust and use both a LSP and Clipy to feed context tot the AI. By doing that The AI gets a tone of context that allows it to better debug and improve the code. 

I find that I can in theory fully automate my AI augmented flow by having a multi-agent workflow, One agent defines product, another does market research, another 2 review the first 2 work. Then a UI/UX agent creates users flows, project manger creates user stores and Behavioer drivent development sensiors then a Senior engineer agent creates the specs and yet angain another 2 review agents review and check there work then the codinging agents build the hole dam thing

#### My usually AI workflow
I usually writhe the specs myself and have AI poke holes in my architecture and I find AI is really good at Code Review. After I define my specs I usually then write the models and database schema myself and then have the AI review it. Same with tests on core Business logic.

I use Behavioer Driven development wiht Spec Driven development to defined how the app should Behave, Look and feel. I use Spec Driven development to define the data structues, controle flow an logic of the app, then I use test driven development to be able to veryify that the app works as intended 

#### AI security fails

The AI secure review bot found 25 security issues and 4 were critical security issues, they were:

- **Command injection** the AI calles the defualt editor env directly using  using subprocces `sh -c "$EDITOR {id}"`. `$EDITOR`, this can contain shell metacharacters that would allow an attacker to reach to the internet via curl. Your box could need to already be poped but RCE is really bad 

- **SQL injection** the AI used string interpolation instead of parameterized bindings. 

- **JWT token stored world-readable** the AI wrought to a Config file at `~/.config/nerdtime/config.toml` written with default umask (typically `0644`). JWT token readable by any process on the system. No `set_permissions()` call, no keyring integration. This means anyone can forge keys and use

- **Hardcoded JWT secrets (dev)**`config/development.yaml:107` and `docker-compose.dev.yml:19` contain literal JWT signing key `WqOAD0KPFoE8YgKw7Ok1`. Anyone with repo access can forge JWTs for dev/staging instances. I junior dev would not make this mistake  


#### AI code quality fails

The Code is horrible spaghetti code. I am going to have to do a full rewrite to make this a production grade app. I wanted it to use a MonoRepo pattern to make managing shared logic easier and the AI still duplicated the logic in several places. 



