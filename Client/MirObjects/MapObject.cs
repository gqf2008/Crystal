using Client.MirControls;
using Client.MirGraphics;
using Client.MirScenes;
using Client.MirSounds;
using Client.MirScenes.Dialogs;
using SlimDX;

namespace Client.MirObjects
{
    /// <summary>
    /// 地图对象基类 - 所有地图上可见对象的抽象基类
    /// 包括: 玩家(User)、怪物(Monster)、NPC、掉落物(Item)、特效(Effect)等
    /// 负责对象的位置管理、动画渲染、状态更新、交互处理等核心功能
    /// </summary>
    public abstract class MapObject
    {
        // ==================== 静态共享资源 ====================
        /// <summary>聊天字体 - 所有对象共享的聊天消息字体</summary>
        public static Font ChatFont = new Font(Settings.FontName, 10F);
        
        /// <summary>标签列表 - 管理所有对象的文字标签(名字/聊天/血条等)</summary>
        public static List<MirLabel> LabelList = new List<MirLabel>();

        // ==================== 静态对象引用 ====================
        /// <summary>当前玩家对象 - 全局唯一的玩家角色</summary>
        public static UserObject User;
        
        /// <summary>玩家英雄对象 - 玩家召唤的英雄伙伴</summary>
        public static UserHeroObject Hero;
        
        /// <summary>英雄对象实例 - 地图上的英雄实体</summary>
        public static HeroObject HeroObject;
        
        /// <summary>鼠标指向对象 - 当前鼠标悬停的对象</summary>
        /// <summary>目标对象 - 当前选中的攻击目标</summary>
        /// <summary>魔法目标对象 - 施法时选中的目标</summary>
        public static MapObject MouseObject, TargetObject, MagicObject;

        // ==================== 鼠标对象ID管理 ====================
        private static uint mouseObjectID;
        /// <summary>
        /// 鼠标对象ID - 鼠标悬停对象的唯一ID
        /// 设置时自动从 MapControl.Objects 查找并更新 MouseObject
        /// </summary>
        public static uint MouseObjectID
        {
            get => mouseObjectID;
            set
            {
                if (mouseObjectID == value) return;
                mouseObjectID = value;
                MouseObject = MapControl.Objects.TryGetValue(value, out var obj) ? obj : null;
            }
        }

        // ==================== 目标对象ID管理 ====================
        private static uint lastTargetObjectId;
        private static uint targetObjectID;
        /// <summary>
        /// 目标对象ID - 当前选中目标的唯一ID
        /// 设置时自动保存上一个目标ID到 lastTargetObjectId，并更新 TargetObject
        /// 用于攻击、查看、交互等需要锁定目标的操作
        /// </summary>
        public static uint TargetObjectID
        {
            get => targetObjectID;
            set
            {
                if (targetObjectID == value) return;
                lastTargetObjectId = value;
                targetObjectID = value;
                TargetObject = MapControl.Objects.TryGetValue(value, out var obj) ? obj : null;
            }
        }

        // ==================== 魔法目标ID管理 ====================
        private static uint magicObjectID;
        /// <summary>
        /// 魔法对象ID - 施法时锁定的目标ID
        /// 设置时自动从 MapControl.Objects 查找并更新 MagicObject
        /// 用于技能施放、治疗、辅助等魔法操作
        /// </summary>
        public static uint MagicObjectID
        {
            get => magicObjectID;
            set
            {
                if (magicObjectID == value) return;
                magicObjectID = value;
                MagicObject = MapControl.Objects.TryGetValue(value, out var obj) ? obj : null;
            }
        }

        // ==================== 抽象属性 ====================
        /// <summary>对象种族类型 - 玩家/怪物/NPC/物品等 (子类必须实现)</summary>
        public abstract ObjectType Race { get; }
        
        /// <summary>是否阻挡移动 - 是否阻止其他对象通过 (子类必须实现)</summary>
        public abstract bool Blocking { get; }

        // ==================== 基础属性 ====================
        /// <summary>对象唯一ID - 服务器分配的全局唯一标识</summary>
        public uint ObjectID;
        
        /// <summary>对象名称 - 显示在对象头顶的名字</summary>
        public string Name = string.Empty;
        
        /// <summary>当前位置 - 对象的实际地图坐标(整数格子坐标)</summary>
        /// <summary>地图位置 - 对象在地图上的坐标(用于某些特殊计算)</summary>
        public Point CurrentLocation, MapLocation;
        
        /// <summary>朝向 - 对象面朝的方向(上下左右及斜向8个方向)</summary>
        public MirDirection Direction;
        
        /// <summary>是否死亡 - 对象是否处于死亡状态</summary>
        /// <summary>是否隐藏 - 对象是否不可见(隐身/潜行)</summary>
        /// <summary>是否坐下 - 对象是否处于坐下状态</summary>
        /// <summary>是否潜行 - 对象是否处于潜行状态(刺客技能)</summary>
        public bool Dead, Hidden, SitDown, Sneaking;
        
        /// <summary>中毒类型 - 对象当前中毒状态(绿毒/红毒/冰冻等)</summary>
        public PoisonType Poison;
        
        /// <summary>死亡时间 - 对象死亡的时间戳</summary>
        public long DeadTime;
        
        /// <summary>AI类型 - 怪物的AI行为类型</summary>
        public byte AI;
        
        /// <summary>是否在陷阱石中 - 对象是否被陷阱石困住</summary>
        public bool InTrapRock;
        
        /// <summary>跳跃距离 - 对象跳跃/位移技能的距离</summary>
        public int JumpDistance;

        /// <summary>是否混合渲染 - 控制对象的透明度混合</summary>
        public bool Blend = true;

        // ==================== 失明状态 ====================
        /// <summary>失明时间 - 失明状态的结束时间</summary>
        public long BlindTime;
        
        /// <summary>失明计数 - 失明效果的叠加层数</summary>
        public byte BlindCount;

        // ==================== 生命值管理 ====================
        private byte percentHealth;
        /// <summary>
        /// 生命值百分比 - 对象当前生命值占最大值的百分比(0-100)
        /// 用于显示血条，变化时触发UI更新
        /// </summary>
        public virtual byte PercentHealth
        {
            get { return percentHealth; }
            set
            {
                if (percentHealth == value) return;
                percentHealth = value;
            }
        }
        
        /// <summary>生命值时间戳 - 生命值最后更新的时间(用于血条显示时长)</summary>
        public long HealthTime;

        // ==================== 魔法值管理 ====================
        private byte percentMana;
        /// <summary>
        /// 魔法值百分比 - 对象当前魔法值占最大值的百分比(0-100)
        /// 用于显示蓝条
        /// </summary>
        public virtual byte PercentMana
        {
            get { return percentMana; }
            set
            {
                if (percentMana == value) return;
                percentMana = value;
            }
        }

        /// <summary>上一个目标ID - 记录上一次选中的目标(用于重新锁定)</summary>
        public uint LastTargetObjectId => lastTargetObjectId;

        // ==================== 动作系统 ====================
        /// <summary>动作队列 - 对象待执行的动作序列(移动/攻击/施法等)</summary>
        public List<QueuedAction> ActionFeed = new List<QueuedAction>();
        
        /// <summary>
        /// 下一个动作 - 获取队列中的第一个待执行动作
        /// 返回null表示队列为空
        /// </summary>
        public QueuedAction NextAction
        {
            get { return ActionFeed.Count > 0 ? ActionFeed[0] : null; }
        }

        // ==================== 特效和增益 ====================
        /// <summary>特效列表 - 对象身上的所有视觉特效(火焰/冰冻/光环等)</summary>
        public List<Effect> Effects = new List<Effect>();
        
        /// <summary>Buff列表 - 对象的所有增益/减益状态(攻击加成/防御加成/减速等)</summary>
        public List<BuffType> Buffs = new List<BuffType>();

        // ==================== 渲染相关 ====================
        /// <summary>身体图库 - 对象身体外观使用的图像库</summary>
        public MLibrary BodyLibrary;
        
        /// <summary>绘制颜色 - 对象的着色(白色=正常, 红色=受击, 灰色=死亡等)</summary>
        /// <summary>名字颜色 - 对象名称的颜色(白色=友好, 红色=敌对, 黄色=NPC等)</summary>
        /// <summary>光照颜色 - 对象发出的光照颜色</summary>
        public Color DrawColour = Color.White, NameColour = Color.White, LightColour = Color.White;
        
        /// <summary>名字标签 - 显示对象名称的UI标签</summary>
        /// <summary>聊天标签 - 显示对象聊天内容的UI标签</summary>
        /// <summary>公会标签 - 显示对象公会名称的UI标签</summary>
        public MirLabel NameLabel, ChatLabel, GuildLabel;
        
        /// <summary>聊天时间 - 聊天消息显示的时间戳(用于自动隐藏)</summary>
        public long ChatTime;
        
        /// <summary>绘制帧 - 当前渲染的动画帧索引</summary>
        /// <summary>翅膀绘制帧 - 翅膀动画的帧索引</summary>
        public int DrawFrame, DrawWingFrame;
        
        /// <summary>绘制位置 - 屏幕上的绘制坐标(像素)</summary>
        /// <summary>移动位置 - 对象移动时的渲染位置(平滑插值)</summary>
        /// <summary>最终绘制位置 - 考虑所有偏移后的最终屏幕坐标</summary>
        /// <summary>偏移移动 - 像素级移动偏移(0-47横向, 0-31纵向)</summary>
        public Point DrawLocation, Movement, FinalDrawLocation, OffSetMove;
        
        /// <summary>显示矩形 - 对象在屏幕上占据的矩形区域(用于鼠标点击检测)</summary>
        public Rectangle DisplayRectangle;
        
        /// <summary>光照强度 - 对象发出的光照强度(0-255)</summary>
        /// <summary>绘制Y坐标 - Y轴排序用的坐标(用于正确的遮挡关系)</summary>
        public int Light, DrawY;
        
        /// <summary>下次动作时间 - 下一次动画帧更新的时间戳</summary>
        /// <summary>下次动作时间2 - 第二个动画计时器(用于复杂动画)</summary>
        public long NextMotion, NextMotion2;
        
        /// <summary>当前动作 - 对象当前执行的动作类型(站立/行走/攻击/施法/死亡等)</summary>
        public MirAction CurrentAction;
        
        /// <summary>当前动作等级 - 动作的变种等级(如不同的攻击动作)</summary>
        public byte CurrentActionLevel;
        
        /// <summary>是否跳帧 - 动画是否跳过某些帧以加快播放</summary>
        public bool SkipFrames;
        
        /// <summary>帧循环 - 动画帧的循环控制器</summary>
        public FrameLoop FrameLoop = null;

        // ==================== 音效 ====================
        /// <summary>受击武器 - 被什么武器击中(用于播放对应的受击音效)</summary>
        public int StruckWeapon;

        // ==================== 临时标签 ====================
        /// <summary>临时标签 - 用于临时显示信息的UI标签</summary>
        public MirLabel TempLabel;

        // ==================== 伤害显示 ====================
        /// <summary>伤害标签列表 - 管理所有伤害数字的UI标签(飘字效果)</summary>
        public static List<MirLabel> DamageLabelList = new List<MirLabel>();
        
        /// <summary>伤害列表 - 对象受到的伤害记录(用于显示伤害数字)</summary>
        public List<Damage> Damages = new List<Damage>();

        /// <summary>
        /// 全局显示位置偏移 - 子类可以重写此属性来调整对象的绘制偏移
        /// 默认返回(0, 0)无偏移
        /// </summary>
        protected Point GlobalDisplayLocationOffset
        {
            get { return new Point(0, 0); }
        }

        protected MapObject() { }

        protected MapObject(uint objectID)
        {
            ObjectID = objectID;

            if (MapControl.Objects.TryGetValue(ObjectID, out var existingObject))
                existingObject.Remove();

            MapControl.Objects[ObjectID] = this;
            MapControl.ObjectsList.Add(this);
            RestoreTargetStates();
        }

        public void Remove()
        {
            if (MouseObject == this) MouseObjectID = 0;
            if (TargetObject == this)
            {
                TargetObjectID = 0;
                lastTargetObjectId = ObjectID;
            }
            if (MagicObject == this) MagicObjectID = 0;

            if (this == User.NextMagicObject)
                User.ClearMagic();

            MapControl.Objects.Remove(ObjectID);
            MapControl.ObjectsList.Remove(this);
            GameScene.Scene.MapControl.RemoveObject(this);

            if (ObjectID == Hero?.ObjectID)
                HeroObject = null;

            if (ObjectID != GameScene.NPCID) return;

            GameScene.NPCID = 0;
            GameScene.Scene.NPCDialog.Hide();
        }

        public abstract void Process();
        public abstract void Draw();
        public abstract bool MouseOver(Point p);

        private void RestoreTargetStates()
        {
            if (MouseObjectID == ObjectID)
                MouseObject = this;

            if (TargetObjectID == ObjectID)
                TargetObject = this;

            if (MagicObjectID == ObjectID)
                MagicObject = this;

            if (!this.Dead &&
                TargetObject == null &&
                LastTargetObjectId == ObjectID)
            {
                switch (Race)
                {
                    case ObjectType.Player:
                    case ObjectType.Monster:
                    case ObjectType.Hero:
                        targetObjectID = ObjectID;
                        TargetObject = this;
                        break;
                }
            }
        }

        public void AddBuffEffect(BuffType type)
        {
            for (int i = 0; i < Effects.Count; i++)
            {
                if (!(Effects[i] is BuffEffect)) continue;
                if (((BuffEffect)(Effects[i])).BuffType == type) return;
            }

            PlayerObject ob = null;

            if (Race == ObjectType.Player)
            {
                ob = (PlayerObject)this;
            }

            switch (type)
            {
                case BuffType.Fury:
                    Effects.Add(new BuffEffect(Libraries.Magic3, 190, 7, 1400, this, true, type) { Repeat = true });
                    break;
                case BuffType.ImmortalSkin:
                    Effects.Add(new BuffEffect(Libraries.Magic3, 570, 5, 1400, this, true, type) { Repeat = true });
                    break;
                case BuffType.SwiftFeet:
                    if (ob != null) ob.Sprint = true;
                    break;
                case BuffType.MoonLight:
                case BuffType.DarkBody:
                    if (ob != null) ob.Sneaking = true;
                    break;
                case BuffType.VampireShot:
                    Effects.Add(new BuffEffect(Libraries.Magic3, 2110, 6, 1400, this, true, type) { Repeat = false });
                    break;
                case BuffType.PoisonShot:
                    Effects.Add(new BuffEffect(Libraries.Magic3, 2310, 7, 1400, this, true, type) { Repeat = false });
                    break;
                case BuffType.EnergyShield:
                    BuffEffect effect;

                    Effects.Add(effect = new BuffEffect(Libraries.Magic2, 1880, 9, 900, this, true, type) { Repeat = false });
                    SoundManager.PlaySound(20000 + (ushort)Spell.EnergyShield * 10 + 0);

                    effect.Complete += (o, e) =>
                    {
                        Effects.Add(new BuffEffect(Libraries.Magic2, 1900, 2, 800, this, true, type) { Repeat = true });
                    };
                    break;
                case BuffType.MagicBooster:
                    Effects.Add(new BuffEffect(Libraries.Magic3, 90, 6, 1200, this, true, type) { Repeat = true });
                    break;
                case BuffType.PetEnhancer:
                    Effects.Add(new BuffEffect(Libraries.Magic3, 230, 6, 1200, this, true, type) { Repeat = true });
                    break;
                case BuffType.GameMaster:
                    Effects.Add(new BuffEffect(Libraries.CHumEffect[5], 0, 1, 1200, this, true, type) { Repeat = true });
                    break;
                case BuffType.GeneralMeowMeowShield:
                    Effects.Add(new BuffEffect(Libraries.Monsters[(ushort)Monster.GeneralMeowMeow], 529, 7, 700, this, true, type) { Repeat = true, Light = 1 });
                    MirSounds.SoundManager.PlaySound(8322);
                    break;
                case BuffType.PowerBeadBuff:
                    Effects.Add(new BuffEffect(Libraries.Monsters[(ushort)Monster.PowerUpBead], 64, 6, 600, this, true, type) { Blend = true, Repeat = true });
                    break;
                case BuffType.HornedArcherBuff:
                    Effects.Add(effect = new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedArcher], 468, 6, 600, this, true, type) { Repeat = false });
                    effect.Complete += (o, e) =>
                    {
                        Effects.Add(new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedArcher], 474, 3, 1000, this, true, type) { Blend = true, Repeat = true });
                    };
                    break;
                case BuffType.ColdArcherBuff:
                    Effects.Add(effect = new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedArcher], 477, 7, 700, this, true, type) { Repeat = false });
                    effect.Complete += (o, e) =>
                    {
                        Effects.Add(new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedArcher], 484, 3, 1000, this, true, type) { Blend = true, Repeat = true });
                    };
                    break;
                case BuffType.HornedWarriorShield:
                    Effects.Add(new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedWarrior], 912, 18, 1800, this, true, type) { Repeat = true });
                    break;
                case BuffType.HornedCommanderShield:
                    Effects.Add(effect = new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedCommander], 1173, 1, 100, this, true, type) { Repeat = false, Light = 1 });
                    effect.Complete += (o, e) =>
                    {
                        Effects.Add(new BuffEffect(Libraries.Monsters[(ushort)Monster.HornedCommander], 1174, 16, 1600, this, true, type) { Repeat = true, Light = 1 });
                    };
                    break;
            }
        }
        public void RemoveBuffEffect(BuffType type)
        {
            PlayerObject ob = null;

            if (Race == ObjectType.Player)
            {
                ob = (PlayerObject)this;
            }

            for (int i = 0; i < Effects.Count; i++)
            {
                if (!(Effects[i] is BuffEffect)) continue;
                if (((BuffEffect)(Effects[i])).BuffType != type) continue;
                Effects[i].Repeat = false;
            }

            switch (type)
            {
                case BuffType.SwiftFeet:
                    if (ob != null) ob.Sprint = false;
                    break;
                case BuffType.MoonLight:
                case BuffType.DarkBody:
                    if (ob != null) ob.Sneaking = false;
                    break;
            }
        }

        public Color ApplyDrawColour()
        {
            Color drawColour = DrawColour;
            if (drawColour == Color.Gray)
            {
                drawColour = Color.White;
                DXManager.SetGrayscale(true);
            }
            return drawColour;
        }

        public virtual Missile CreateProjectile(int baseIndex, MLibrary library, bool blend, int count, int interval, int skip, int lightDistance = 6, bool direction16 = true, Color? lightColour = null, uint targetID = 0)
        {
            return null;
        }

        public void Chat(string text)
        {
            if (ChatLabel != null && !ChatLabel.IsDisposed)
            {
                ChatLabel.Dispose();
                ChatLabel = null;
            }

            const int chatWidth = 200;
            List<string> chat = new List<string>();

            int index = 0;
            for (int i = 1; i < text.Length; i++)
                if (TextRenderer.MeasureText(CMain.Graphics, text.Substring(index, i - index), ChatFont).Width > chatWidth)
                {
                    chat.Add(text.Substring(index, i - index - 1));
                    index = i - 1;
                }
            chat.Add(text.Substring(index, text.Length - index));

            text = chat[0];
            for (int i = 1; i < chat.Count; i++)
                text += string.Format("\n{0}", chat[i]);

            ChatLabel = new MirLabel
            {
                AutoSize = true,
                BackColour = Color.Transparent,
                ForeColour = Color.White,
                OutLine = true,
                OutLineColour = Color.Black,
                DrawFormat = TextFormatFlags.HorizontalCenter,
                Text = text,
            };
            ChatTime = CMain.Time + 5000;
        }
        public virtual void DrawChat()
        {
            if (ChatLabel == null || ChatLabel.IsDisposed) return;

            if (CMain.Time > ChatTime)
            {
                ChatLabel.Dispose();
                ChatLabel = null;
                return;
            }

            ChatLabel.ForeColour = Dead ? Color.Gray : Color.White;
            ChatLabel.Location = new Point(DisplayRectangle.X + (48 - ChatLabel.Size.Width) / 2, DisplayRectangle.Y - (60 + ChatLabel.Size.Height) - (Dead ? 35 : 0));
            ChatLabel.Draw();
        }

        public virtual void CreateLabel()
        {
            NameLabel = null;

            for (int i = 0; i < LabelList.Count; i++)
            {
                if (LabelList[i].Text != Name || LabelList[i].ForeColour != NameColour) continue;
                NameLabel = LabelList[i];
                break;
            }


            if (NameLabel != null && !NameLabel.IsDisposed) return;

            NameLabel = new MirLabel
            {
                AutoSize = true,
                BackColour = Color.Transparent,
                ForeColour = NameColour,
                OutLine = true,
                OutLineColour = Color.Black,
                Text = Name,
            };
            NameLabel.Disposing += (o, e) => LabelList.Remove(NameLabel);
            LabelList.Add(NameLabel);



        }
        public virtual void DrawName()
        {
            CreateLabel();

            if (NameLabel == null) return;

            //NameLabel.Text = Name; //When CreateLabel() is called, the name is already determined, so there's no need to assign it every time in DrawName.
            NameLabel.Location = new Point(DisplayRectangle.X + (50 - NameLabel.Size.Width) / 2, DisplayRectangle.Y - (32 - NameLabel.Size.Height / 2) + (Dead ? 35 : 8)); //was 48 -
            NameLabel.Draw();
        }
        public virtual void DrawBlend()
        {
            DXManager.SetBlend(true, 0.3F); //0.8
            Draw();
            DXManager.SetBlend(false);
        }
        public void DrawDamages()
        {
            for (int i = Damages.Count - 1; i >= 0; i--)
            {
                Damage info = Damages[i];
                if (CMain.Time > info.ExpireTime)
                {
                    if (info.DamageLabel != null)
                    {
                        info.DamageLabel.Dispose();
                    }

                    Damages.RemoveAt(i);
                }
                else
                {
                    info.Draw(DisplayRectangle.Location);
                }
            }
        }
        public virtual bool ShouldDrawHealth()
        {
            return false;
        }
        public void DrawHealth()
        {
            string name = Name;
            if (Name.Contains("(")) name = Name.Substring(Name.IndexOf("(") + 1, Name.Length - Name.IndexOf("(") - 2);

            if (Dead) return;
            if (Race != ObjectType.Player && Race != ObjectType.Monster && Race != ObjectType.Hero) return;

            if (CMain.Time >= HealthTime)
            {
                if (!ShouldDrawHealth()) return;
            }

            Libraries.Prguse2.Draw(0, DisplayRectangle.X + 8, DisplayRectangle.Y - 64);
            int index = 1;

            switch (Race)
            {
                case ObjectType.Player:
                    index = 12;
                    if (GroupDialog.GroupList.Contains(name) && name != User.Name) index = 10;
                    break;
                case ObjectType.Monster:
                    if (GroupDialog.GroupList.Contains(name) || name == User.Name) index = 11;
                    break;
                case ObjectType.Hero:
                    if (GroupDialog.GroupList.Contains(MapObject.HeroObject?.OwnerName)) // Fails but not game breaking
                    {
                        index = 11;
                    }
                    if (HeroObject.HeroObject?.OwnerName == User.Name)
                    {
                        index = 1;
                        if ((MapObject.HeroObject.Class != MirClass.Warrior && HeroObject.Level > 7) || (MapObject.HeroObject.Class == MirClass.Warrior && HeroObject.Level > 25))
                        {
                            Libraries.Prguse2.Draw(10, new Rectangle(0, 0, (int)(32 * PercentMana / 100F), 4), new Point(DisplayRectangle.X + 8, DisplayRectangle.Y - 60), Color.White, false);
                        }
                    }
                    break;
            }

            Libraries.Prguse2.Draw(index, new Rectangle(0, 0, (int)(32 * PercentHealth / 100F), 4), new Point(DisplayRectangle.X + 8, DisplayRectangle.Y - 64), Color.White, false);
        }

        public void DrawPoison()
        {
            byte poisoncount = 0;
            if (Poison != PoisonType.None)
            {
                if (Poison.HasFlag(PoisonType.Green))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Green);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Red))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Red);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Bleeding))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.DarkRed);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Slow))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Purple);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Stun) || Poison.HasFlag(PoisonType.Dazed))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Yellow);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Blindness))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.MediumVioletRed);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Frozen))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Blue);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.Paralysis) || Poison.HasFlag(PoisonType.LRParalysis))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Gray);
                    poisoncount++;
                }
                if (Poison.HasFlag(PoisonType.DelayedExplosion))
                {
                    DXManager.Draw(DXManager.PoisonDotBackground, new Rectangle(0, 0, 6, 6), new Vector3((float)(DisplayRectangle.X + 7 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 21), 0.0F), Color.Black);
                    DXManager.Draw(DXManager.RadarTexture, new Rectangle(0, 0, 4, 4), new Vector3((float)(DisplayRectangle.X + 8 + (poisoncount * 5)), (float)(DisplayRectangle.Y - 20), 0.0F), Color.Orange);
                    poisoncount++;
                }
            }
        }

        public abstract void DrawBehindEffects(bool effectsEnabled);

        public abstract void DrawEffects(bool effectsEnabled);

        protected void LoopFrame(int start, int frameCount, int frameInterval, int duration)
        {
            if (FrameLoop == null)
            {
                FrameLoop = new FrameLoop
                {
                    Start = start,
                    End = start + frameCount - 1,
                    Loops = (duration / (frameInterval * frameCount)) - 1 //Remove 1 count as we've already done a loop before this is checked
                };
            }
        }
    }

    public class FrameLoop
    {
        public MirAction Action { get; set; }
        public int Start { get; set; }
        public int End { get; set; }
        public int Loops { get; set; }

        public int CurrentCount { get; set; }
    }

}
