using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;

class Program
{
    static string Root;
    static Type[] allTypes;

    static object F(object o, string name)
    {
        if (o == null) return null;
        var t = o.GetType();
        var f = t.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static);
        if (f != null) return f.GetValue(o);
        var p = t.GetProperty(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static);
        if (p != null) return p.GetValue(o);
        return null;
    }

    static IEnumerable<object> Seq(object o)
    {
        if (o is IEnumerable e && !(o is string)) foreach (var x in e) yield return x;
    }

    static string S(object o) => o?.ToString() ?? "";
    static long I(object o) => o == null ? 0 : Convert.ToInt64(o);
    static string Esc(string s) => s.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", " ").Replace("\n", " ");

    static void Main(string[] args)
    {
        // argv: <serverDir> <mode> [arg]   e.g. <serverDir> export out.json
        Root = Path.GetFullPath(args.Length > 0 ? args[0] : ".") + Path.DirectorySeparatorChar;
        Directory.SetCurrentDirectory(Root);   // the original Envir resolves Server.MirDB / Server.MirADB relative to CWD
        AppDomain.CurrentDomain.AssemblyResolve += (s, e) =>
        {
            var name = new AssemblyName(e.Name).Name + ".dll";
            var p = Path.Combine(Root, name);
            return File.Exists(p) ? Assembly.LoadFrom(p) : null;
        };
        var mode = args.Length > 1 ? args[1] : "list";
        var arg2 = args.Length > 2 ? args[2] : null;
        var lib = Assembly.LoadFrom(Root + "Server.Library.dll");
        var shared = Assembly.LoadFrom(Root + "Shared.dll");
        allTypes = SafeTypes(lib).Concat(SafeTypes(shared)).ToArray();
        Console.WriteLine($"server dir: {Root}");
        Console.WriteLine($"loaded Server.Library ({allTypes.Length} types incl. Shared)");

        if (mode == "dump")
        {
            var t = allTypes.FirstOrDefault(x => x.FullName == arg2);
            if (t == null) { Console.WriteLine("not found: " + arg2); return; }
            foreach (var f in t.GetFields(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static))
                Console.WriteLine("F " + (f.IsStatic ? "static " : "") + f.FieldType.Name + " " + f.Name);
            foreach (var p in t.GetProperties(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static))
                Console.WriteLine("P " + p.PropertyType.Name + " " + p.Name);
            return;
        }
        if (mode == "list")
        {
            foreach (var t in allTypes.OrderBy(x => x.FullName)) Console.WriteLine(t.FullName);
            return;
        }
        if (mode == "export") { Export(arg2 ?? "db_export.json"); return; }
        if (mode == "setpw") { SetPassword(arg2, args.Length > 3 ? args[3] : null); return; }
        if (mode == "setgold") { SetGold(arg2, args.Length > 3 ? args[3] : null, args.Length > 4 ? args[4] : null); return; }
        if (mode == "npcs") { ListNpcs(arg2); return; }
        if (mode == "gameshop") { ListGameShop(arg2); return; }
        if (mode == "setpos")
        {
            // setpos <accountId> <mapIndex> <x> <y> [charName]
            //   把账号下（或指定）角色的落点改掉，只重写 Server.MirADB（与 setpw 同一条保存路径）。
            //   用途：给沙箱一个**重置按钮** —— 原版侧那些"点到哪只 NPC"的取证要靠格点扫描，
            //   而扫描本身会把角色带走；每次先 setpos 回同一个格子，扫描/复现才有确定性。
            SetPos(arg2,
                   args.Length > 3 ? args[3] : null,
                   args.Length > 4 ? args[4] : null,
                   args.Length > 5 ? args[5] : null,
                   args.Length > 6 ? args[6] : null);
            return;
        }
        Console.WriteLine("usage: dbtool <serverDir> list | dump <TypeFullName> | export <outfile> | " +
                          "setpw <accountId> <newPassword> | setgold <accountId> <gold> [credit] | " +
                          "npcs [mapIndex] | gameshop [outfile] | " +
                          "setpos <accountId> <mapIndex> <x> <y> [charName]");
    }

    // 商城分类对账：`GameShopList`（Server.MirDB）里的**分类名与条数**。
    // 用途：客户端 `GameshopDialog` 的分类列滚动/滑条行程要 `CategoryList.Count > 22`
    // （`GameshopDialog.cs:135/583` 两处守卫），而 README §3.2y 当时是拿一份 **Drops** 里的
    // `GameShop_Guard.txt`（10 个以分类命名的段）当"商城数据"，那多半不是真源 —— 这条把真源量出来。
    static void ListGameShop(string outArg)
    {
        var env = LoadEnvir(out _);
        // ⚠️ `BindGameShop(item)` 是拿 **`Envir.Edit.ItemInfoList`**（编辑器那份）去绑 ItemInfo 的
        // （`Server/MirEnvir/Envir.cs:4550-4561`），而离线 `LoadDB()` 里那份是**空的**
        // ⇒ 每个商品都会 `return false` 被丢掉，`GameShopList` 恒 0（实测就是这个原因，
        // 不是"DB 里没有商城"）。把 `ItemInfoList` 灌进 `Edit` 再跑一遍 LoadDB，这一遍才量得到真值。
        SecondPassForEditBoundLists(env);
        var items = Seq(F(env, "GameShopList")).ToList();
        var byCat = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var it in items)
        {
            var cat = S(F(it, "Category"));
            if (cat.Length == 0) cat = "(none)";
            byCat[cat] = byCat.TryGetValue(cat, out var n) ? n + 1 : 1;
        }
        var cats = byCat.Keys.ToList();
        cats.Sort(StringComparer.Ordinal);

        var sb = new StringBuilder();
        sb.Append("{\n");
        sb.Append("  \"source\": \"" + Esc(Root) + "\",\n");
        sb.Append("  \"items\": " + items.Count + ",\n");
        sb.Append("  \"categories\": " + cats.Count + ",\n");
        sb.Append("  \"categoryListCountOver22\": " + (cats.Count > 22 ? "true" : "false") + ",\n");
        sb.Append("  \"categoryList\": [\n");
        sb.Append(string.Join(",\n", cats.ConvertAll(c =>
            "    {\"name\":\"" + Esc(c) + "\",\"items\":" + byCat[c] + "}")));
        sb.Append("\n  ]\n}\n");

        var outFile = string.IsNullOrEmpty(outArg)
            ? Path.Combine(Path.GetFullPath(Path.Combine(Root, "..")), "gameshop.json")
            : Path.GetFullPath(outArg);
        File.WriteAllText(outFile, sb.ToString(), new UTF8Encoding(false));
        Console.WriteLine($"gameshop: {items.Count} item(s), {cats.Count} categor(ies), " +
                          $"Count>22={cats.Count > 22} → {outFile}");
        foreach (var c in cats) Console.WriteLine($"  {c} : {byCat[c]}");
    }

    // 列出 Server.MirDB 的 NPCInfoList（`FileName` = 脚本相对路径、`Location` = 世界格），可按地图过滤。
    // 用途：把原版侧的点击取证从「盲扫格点」升级成「按坐标点」——拿到 FileName 就能先判这一只
    // 是不是商人（脚本里有 `<View/@BuySell>` 一类），再用 GameScene 的世界→屏幕公式算落点。
    static void ListNpcs(string mapS)
    {
        var env = LoadEnvir(out _);
        int? map = string.IsNullOrEmpty(mapS) ? (int?)null : int.Parse(mapS);
        var rows = new List<string>();
        foreach (var n in Seq(F(env, "NPCInfoList")))
        {
            int mi = (int)I(F(n, "MapIndex"));
            if (map.HasValue && mi != map.Value) continue;
            var loc = F(n, "Location");
            rows.Add("  {\"index\":" + I(F(n, "Index")) +
                     ",\"fileName\":\"" + Esc(S(F(n, "FileName"))) + "\"" +
                     ",\"name\":\"" + Esc(S(F(n, "Name"))) + "\"" +
                     ",\"mapIndex\":" + mi +
                     ",\"x\":" + I(F(loc, "X")) + ",\"y\":" + I(F(loc, "Y")) +
                     ",\"image\":" + I(F(n, "Image")) + ",\"rate\":" + I(F(n, "Rate")) + "}");
        }
        var sb = new StringBuilder();
        sb.Append("[\n" + string.Join(",\n", rows) + "\n]\n");
        var outFile = Path.Combine(Root, "..", "npcs_" + (map.HasValue ? map.Value.ToString() : "all") + ".json");
        outFile = Path.GetFullPath(outFile);
        File.WriteAllText(outFile, sb.ToString(), new UTF8Encoding(false));
        Console.WriteLine($"npcs(map={(map.HasValue ? map.Value.ToString() : "all")}): {rows.Count} → {outFile}");
    }

    // 改角色落点：Server.MirDatabase.CharacterInfo 的 CurrentMapIndex(Int32) / CurrentLocation(Point)。
    // 只调 SaveAccounts()（写 Server.MirADB）——**不要**碰 SaveDB()：离线 LoadDB() 时 MapInfoList/ItemInfoList
    // 是空的，SaveDB() 会把 Server.MirDB 写坏（dbtool 里的 ProtectGameDb() 就是为此存在的）。
    static void SetPos(string accountId, string mapS, string xS, string yS, string charName)
    {
        if (string.IsNullOrEmpty(accountId) || string.IsNullOrEmpty(mapS) ||
            string.IsNullOrEmpty(xS) || string.IsNullOrEmpty(yS))
        {
            Console.WriteLine("usage: setpos <accountId> <mapIndex> <x> <y> [charName]");
            return;
        }
        int map = int.Parse(mapS), x = int.Parse(xS), y = int.Parse(yS);
        var env = LoadEnvir(out var err);
        if (err.Length > 0) { Console.WriteLine("accounts did not load: " + err); return; }
        var target = Seq(F(env, "AccountList")).FirstOrDefault(a => S(F(a, "AccountID")) == accountId);
        if (target == null) { Console.WriteLine("account not found: " + accountId); return; }

        int changed = 0;
        foreach (var ch in Seq(F(target, "Characters")))
        {
            var name = S(F(ch, "Name"));
            if (!string.IsNullOrEmpty(charName) && name != charName) continue;
            var t = ch.GetType();
            var mapField = t.GetField("CurrentMapIndex");
            var locField = t.GetField("CurrentLocation");
            if (mapField == null || locField == null)
            {
                Console.WriteLine($"character {name}: CurrentMapIndex/CurrentLocation field not found");
                continue;
            }
            var before = $"map={I(F(ch, "CurrentMapIndex"))} loc={S(F(ch, "CurrentLocation"))}";
            mapField.SetValue(ch, map);
            locField.SetValue(ch, Activator.CreateInstance(locField.FieldType, x, y));
            Console.WriteLine($"character {name}: {before} -> map={map} loc={{X={x},Y={y}}}");
            changed++;
        }
        if (changed == 0) { Console.WriteLine("no matching character on account " + accountId); return; }

        var envirType = allTypes.First(t => t.FullName == "Server.MirEnvir.Envir");
        var saveAcc = envirType.GetMethod("SaveAccounts", BindingFlags.Public | BindingFlags.NonPublic |
                                                           BindingFlags.Instance, null, Type.EmptyTypes, null);
        if (saveAcc == null) { Console.WriteLine("SaveAccounts() not found"); return; }
        saveAcc.Invoke(env, null);
        Console.WriteLine($"saved Server.MirADB ({changed} character(s) updated)");
    }

    static Type[] SafeTypes(Assembly a)
    {
        try { return a.GetTypes(); }
        catch (ReflectionTypeLoadException ex) { return ex.Types.Where(t => t != null).ToArray(); }
    }

    static string ItemList(IEnumerable<object> items, int take, Dictionary<long, string> nameByIndex)
    {
        var l = new List<string>();
        foreach (var it in items.Take(take))
        {
            var ii = I(F(it, "ItemIndex"));
            l.Add($"{{\"itemIndex\":{ii},\"name\":\"{Esc(nameByIndex.TryGetValue(ii, out var n) ? n : "?")}\"," +
                  $"\"uniqueId\":{I(F(it, "UniqueID"))},\"dura\":[{I(F(it, "CurrentDura"))},{I(F(it, "MaxDura"))}],\"count\":{I(F(it, "Count"))}}}");
        }
        return "[" + string.Join(",", l) + "]";
    }

    static void Export(string outFile)
    {
        var env = LoadEnvir(out var loadAccountsError);
        SecondPassForEditBoundLists(env);
        WriteExport(env, loadAccountsError, outFile);
    }

    // 离线 `LoadDB()` 里 `Edit.ItemInfoList` 是空的，而 `BindGameShop()` 之类的绑定读的正是**它**
    // （`Envir.cs:4550-4561`）⇒ 走 `Edit.*` 的列表会全被丢掉，`dbCounts` 里表现为**假零**
    // （`GameShopList` 实测：pass1=0、pass2=105）。把 `ItemInfoList` 灌进 `Edit` 再跑一遍 `LoadDB()`
    // 就修好了；绑定失败的条目不会进 `GameShopList`，所以这一遍的数字才是真值。
    static void SecondPassForEditBoundLists(object env)
    {
        var editItems = F(F(env, "Edit"), "ItemInfoList") as IList;
        if (editItems == null) return;
        editItems.Clear();
        foreach (var info in Seq(F(env, "ItemInfoList"))) editItems.Add(info);
        var loadDb = env.GetType().GetMethod("LoadDB", BindingFlags.Public | BindingFlags.NonPublic |
                                                      BindingFlags.Instance, null, Type.EmptyTypes, null);
        if (loadDb == null) return;
        ProtectGameDb();
        Console.WriteLine("LoadDB() pass 2（Edit.ItemInfoList=" + editItems.Count + "）: " + S(loadDb.Invoke(env, null)));
        ProtectGameDb();
    }

    // Give an existing sandbox account a known password, using the original AccountInfo.Password setter
    // (same hashing the server uses). Only run this against a SANDBOX copy of Server.MirADB.
    static void SetPassword(string accountId, string newPassword)
    {
        if (string.IsNullOrEmpty(accountId) || string.IsNullOrEmpty(newPassword))
        {
            Console.WriteLine("usage: setpw <accountId> <newPassword>");
            return;
        }
        var env = LoadEnvir(out var err);
        if (err.Length > 0) { Console.WriteLine("accounts did not load: " + err); return; }
        var target = Seq(F(env, "AccountList")).FirstOrDefault(a => S(F(a, "AccountID")) == accountId);
        if (target == null) { Console.WriteLine("account not found: " + accountId); return; }
        var before = S(F(target, "password"));
        target.GetType().GetProperty("Password")?.SetValue(target, newPassword);
        var after = S(F(target, "password"));
        Console.WriteLine($"account {accountId}: password hash changed = {before != after}; chars=" +
            Seq(F(target, "Characters")).Count());
        var envirType = allTypes.First(t => t.FullName == "Server.MirEnvir.Envir");
        var saveAcc = envirType.GetMethod("SaveAccounts", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance,
                                          null, Type.EmptyTypes, null);
        if (saveAcc == null) { Console.WriteLine("SaveAccounts() not found"); return; }
        saveAcc.Invoke(env, null);
        Console.WriteLine("saved Server.MirADB");
    }

    // 给账号发钱：`Server.MirDatabase.AccountInfo.Gold` / `.Credit`（两个都是 `UInt32` 公开字段）。
    // 用途：客户端的 `GameScene.Gold/Credit` 就来自这里，而 `MirGameShopCell.BuyProduct()`
    // （`Client/MirControls/MirGameShopCell.cs:193-228`）要过 `Item.GoldPrice * Quantity <= GameScene.Gold`
    // 才会弹 `MirMessageBox(ConfirmPurchaseItemGold, YesNo)`——金币 0 时**只发系统聊天**、不弹框，
    // 于是沙箱里点十次买钮也取不到那扇"原版同状态帧"（见 README §3.2bm）。
    // 只调 `SaveAccounts()`（写 Server.MirADB），**不碰 `SaveDB()`**——与 setpw/setpos 同一条保存路径
    //（离线 LoadDB() 时 MapInfoList/ItemInfoList 是空的，SaveDB() 会把 Server.MirDB 写坏）。
    static void SetGold(string accountId, string goldS, string creditS)
    {
        if (string.IsNullOrEmpty(accountId) || string.IsNullOrEmpty(goldS))
        {
            Console.WriteLine("usage: setgold <accountId> <gold> [credit]");
            return;
        }
        if (!uint.TryParse(goldS, out var gold))
        {
            Console.WriteLine("gold must be a uint32");
            return;
        }
        uint credit = 0u;
        if (!string.IsNullOrEmpty(creditS) && !uint.TryParse(creditS, out credit))
        {
            Console.WriteLine("credit must be a uint32");
            return;
        }
        var env = LoadEnvir(out var err);
        if (err.Length > 0) { Console.WriteLine("accounts did not load: " + err); return; }
        var target = Seq(F(env, "AccountList")).FirstOrDefault(a => S(F(a, "AccountID")) == accountId);
        if (target == null) { Console.WriteLine("account not found: " + accountId); return; }
        var goldField = target.GetType().GetField("Gold", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        var creditField = target.GetType().GetField("Credit", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        if (goldField == null || creditField == null) { Console.WriteLine("Gold/Credit field not found on " + target.GetType().FullName); return; }
        Console.WriteLine($"account {accountId}: gold {I(F(target, "Gold"))} -> {gold}, credit {I(F(target, "Credit"))} -> {credit}");
        goldField.SetValue(target, gold);
        creditField.SetValue(target, credit);
        CheckWalletSavePath(env, target, accountId);
    }

    // 写 Server.MirADB（与 setpw/setpos 同一条路径），写完回读一遍确认落盘值。
    static void CheckWalletSavePath(object env, object target, string accountId)
    {
        var envirType = allTypes.First(t => t.FullName == "Server.MirEnvir.Envir");
        var saveAcc = envirType.GetMethod("SaveAccounts", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance,
                                          null, Type.EmptyTypes, null);
        if (saveAcc == null) { Console.WriteLine("SaveAccounts() not found"); return; }
        saveAcc.Invoke(env, null);
        Console.WriteLine("saved Server.MirADB");
        Console.WriteLine($"readback: {accountId} gold={I(F(target, "Gold"))} credit={I(F(target, "Credit"))}");
    }

    static object LoadEnvir(out string loadAccountsError)
    {
        // Guard: the original Envir rewrites Server.MirDB (the *game* DB) during LoadDB()/init, even
        // though this offline path cannot populate it (0 maps/items). Keep the shipped bytes.
        ProtectGameDb();
        var envirType = allTypes.First(t => t.FullName == "Server.MirEnvir.Envir");
        object env = envirType.GetProperty("Main", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)?.GetValue(null)
                  ?? envirType.GetField("Main", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)?.GetValue(null)
                  ?? Activator.CreateInstance(envirType, true);
        Console.WriteLine("envir instance: " + (env == null ? "NULL" : "ok"));

        var loadDb = envirType.GetMethod("LoadDB", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        var loadAcc = envirType.GetMethod("LoadAccounts", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        Console.WriteLine("LoadDB(): " + S(loadDb?.Invoke(env, null)));
        loadAccountsError = "";
        // The provided MirADB stores characters carrying BuffType.GameMaster, which this build's
        // Envir.GetBuffInfo() does not implement. Pre-register a BuffInfo so account loading can proceed.
        try
        {
            var bt = allTypes.First(t => t.FullName == "BuffType");
            var biType = allTypes.First(t => t.FullName == "Server.MirDatabase.BuffInfo");
            var list = (IList)F(env, "BuffInfoList");
            var existing = new HashSet<string>(Seq(list).Select(x => S(F(x, "Type"))));
            var added = 0;
            foreach (var v in Enum.GetValues(bt))
            {
                if (existing.Contains(v.ToString())) continue;
                var bi = Activator.CreateInstance(biType);
                biType.GetProperty("Type")?.SetValue(bi, v);
                list.Add(bi);
                added++;
            }
            Console.WriteLine($"pre-registered {added} BuffInfo entries (existing {existing.Count})");
        }
        catch (Exception ex) { Console.WriteLine("buff pre-register failed: " + ex.Message); }
        if (loadAcc != null)
        {
            try { loadAcc.Invoke(env, null); Console.WriteLine("LoadAccounts() done"); }
            catch (TargetInvocationException tie)
            {
                loadAccountsError = tie.InnerException?.Message ?? tie.Message;
                Console.WriteLine("LoadAccounts() FAILED: " + loadAccountsError);
            }
        }
        ProtectGameDb();
        return env;
    }

    static void ProtectGameDb()
    {
        var db = Path.Combine(Root, "Server.MirDB");
        var bak = Path.Combine(Root, "Server.MirDB.offline-bak");
        if (File.Exists(bak)) { File.Copy(bak, db, true); return; }
        if (File.Exists(db))
        {
            File.Copy(db, bak, true);
            Console.WriteLine($"Server.MirDB backed up ({new FileInfo(bak).Length} bytes)");
        }
    }

    static void WriteExport(object env, string loadAccountsError, string outFile)
    {
        var itemInfos = Seq(F(env, "ItemInfoList")).ToList();
        var nameByIndex = new Dictionary<long, string>();
        foreach (var ii in itemInfos)
        {
            var idx = F(ii, "Index");
            if (idx != null) nameByIndex[I(idx)] = S(F(ii, "Name"));
        }

        var sb = new StringBuilder();
        sb.Append("{\n");
        sb.Append("  \"source\": \"" + Esc(Root) + "\",\n");
        sb.Append("  \"loadAccountsError\": \"" + Esc(loadAccountsError) + "\",\n");
        var countFields = new[] { "MapInfoList", "ItemInfoList", "MonsterInfoList", "MagicInfoList", "NPCInfoList", "QuestInfoList",
                                  "GameShopList", "RecipeInfoList", "BuffInfoList", "ConquestInfoList", "AccountList", "CharacterList",
                                  "GuildList", "HeroList", "StartItems", "GTMapList" };
        sb.Append("  \"dbCounts\": {\n");
        sb.Append(string.Join(",\n", countFields.Select(f => $"    \"{f}\": {Seq(F(env, f)).Count()}")));
        sb.Append("\n  },\n");

        var accounts = Seq(F(env, "AccountList")).ToList();
        var accLines = new List<string>();
        foreach (var a in accounts)
        {
            var chars = Seq(F(a, "Characters")).Select(c => "\"" + Esc(S(F(c, "Name"))) + "\"").ToList();
            var storage = Seq(F(a, "Storage")).Where(x => x != null).ToList();
            accLines.Add("    {\"index\":" + I(F(a, "Index")) + ",\"accountId\":\"" + Esc(S(F(a, "AccountID"))) + "\",\"userName\":\"" + Esc(S(F(a, "UserName"))) +
                         "\",\"gold\":" + I(F(a, "Gold")) + ",\"credit\":" + I(F(a, "Credit")) + ",\"admin\":" + S(F(a, "AdminAccount")).ToLower() +
                         ",\"characters\":[" + string.Join(",", chars) + "],\"storageItems\":" + storage.Count +
                         ",\"storageSample\":" + ItemList(storage, 10, nameByIndex) + "}");
        }
        sb.Append("  \"accountCount\": " + accounts.Count + ",\n  \"accounts\": [\n");
        sb.Append(string.Join(",\n", accLines));
        sb.Append("\n  ],\n");

        var chars2 = Seq(F(env, "CharacterList")).ToList();
        var charLines = new List<string>();
        foreach (var c in chars2)
        {
            var inv = Seq(F(c, "Inventory")).Where(x => x != null).ToList();
            var magics = Seq(F(c, "Magics")).Select(m => "\"" + Esc(S(F(m, "Spell"))) + "@" + S(F(m, "Level")) + "\"").ToList();
            var pets = Seq(F(c, "Pets")).Select(p => "\"" + Esc(S(F(p, "Name"))) + "\"").ToList();
            var quests = Seq(F(c, "CurrentQuests")).Select(q => I(F(q, "Index")).ToString()).ToList();
            var done = Seq(F(c, "CompletedQuests")).Select(q => I(F(q, "Index")).ToString()).ToList();
            var heroes = Seq(F(c, "Heroes")).Select(h => "\"" + Esc(S(F(h, "Name"))) + "\"").ToList();
            charLines.Add("    {\"index\":" + I(F(c, "Index")) + ",\"name\":\"" + Esc(S(F(c, "Name"))) + "\",\"level\":" + I(F(c, "Level")) +
                ",\"class\":\"" + Esc(S(F(c, "Class"))) + "\",\"gender\":\"" + Esc(S(F(c, "Gender"))) + "\",\"hair\":" + I(F(c, "Hair")) +
                ",\"mapIndex\":" + I(F(c, "CurrentMapIndex")) + ",\"loc\":\"" + Esc(S(F(c, "CurrentLocation"))) + "\",\"hp\":" + I(F(c, "HP")) +
                ",\"mp\":" + I(F(c, "MP")) + ",\"exp\":" + I(F(c, "Experience")) + ",\"pkPoints\":" + I(F(c, "PKPoints")) +
                ",\"bindMapIndex\":" + I(F(c, "BindMapIndex")) + ",\"bindLoc\":\"" + Esc(S(F(c, "BindLocation"))) + "\"" +
                ",\"lastLogin\":\"" + Esc(S(F(c, "LastLoginDate"))) + "\",\"lastLogout\":\"" + Esc(S(F(c, "LastLogoutDate"))) + "\"" +
                ",\"inventoryCount\":" + inv.Count + ",\"inventory\":" + ItemList(inv, 60, nameByIndex) +
                ",\"equipment\":" + ItemList(Seq(F(c, "Equipment")).Where(x => x != null), 20, nameByIndex) +
                ",\"questInventory\":" + ItemList(Seq(F(c, "QuestInventory")).Where(x => x != null), 20, nameByIndex) +
                ",\"magics\":[" + string.Join(",", magics) + "],\"pets\":[" + string.Join(",", pets) + "]" +
                ",\"heroes\":[" + string.Join(",", heroes) + "],\"mail\":" + Seq(F(c, "Mail")).Count() +
                ",\"currentQuests\":[" + string.Join(",", quests) + "],\"completedQuests\":[" + string.Join(",", done) + "]}");
        }
        sb.Append("  \"characterCount\": " + chars2.Count + ",\n  \"characters\": [\n");
        sb.Append(string.Join(",\n", charLines));
        sb.Append("\n  ]\n}\n");
        File.WriteAllText(outFile, sb.ToString(), new UTF8Encoding(false));
        Console.WriteLine($"wrote {outFile}; accounts={accounts.Count} characters={chars2.Count}");
    }
}
