#set document(title: "Example Project 2")
#let scenes = state("scenes", ())

#let fps_state = state("fps", 30)

#let scene_state = state("scene", (
	objects: (),
	background: white,
))

#let scene(name, actions, background: white) = {
	pagebreak(weak: true)
	context {
		let start = counter(page).get().at(0)
		scenes.update(scenes => {
			scenes.push((
				name,
				start,
			))
			scenes
		})
	}

	scene_state.update((objects: (), background: background))

	actions

	let end = context counter(page).get().at(0)

	context {
		let end = counter(page).get().at(0)
		scenes.update(scenes => {
			scenes.last().push(end)
			scenes
		})
	}
}

#let step() = {
	context {
		pagebreak(weak: true)
		let scene = scene_state.get()
		rect(width: 100%, height: 100%, fill: scene.background, stroke: none, inset: 0pt, {
			for object in scene.objects {
				(object.render)(object)
			}
		})
	}
	context {
		let fps = fps_state.get()
		scene_state.update(scene => {
			let new-objects = ()
			for object in scene.objects {
				let object = (object.update)(object, 1.0 / fps)
				if (object.alive)(object) {
					new-objects.push(object)
				}
			}
			scene.objects = new-objects
			scene
		})
	}
}

#let setup(width: 1920pt, height: 1080pt, fps: 30) = {
	(doc) => {
		set page(width: width, height: height, margin: 0pt)
		show raw: set text(font: "FiraCode Nerd Font Mono")
		fps_state.update(fps)
		context [
			#metadata(fps) <fps>

			#metadata(scenes.final()) <final-scenes>
		]
		set text(size: 100pt)
		doc

	}
}

#let spawn(object) = scene_state.update(scene => {
	scene.objects.push(object)
	scene
})

#let object(content) = (
	render: (self) => place(content),
	alive: (self) => true,
	update: (self, delta) => self,
)

#let timed(object, time) = (
	object: object,
	time: time,
	render: (self) => (self.object.render)(self.object),
	alive: (self) => self.time >= 0.0 and (self.object.alive)(self.object),
	update: (self, delta) => {
		self.time -= delta
		self.object = (self.object.update)(self.object, delta)
		self
	},
)

#let translate(x: 0pt, y: 0pt, object) = (
	x: x,
	y: y,
	object: object,
	render: (self) => place(dx: self.x, dy: self.y, (self.object.render)(self.object)),
	alive: (self) => (self.object.alive)(self.object),
	update: (self, delta) => {
		self.object = (self.object.update)(self.object, delta)
		self
	},
)

#let KEEP = "K"
#let DELETE = "D"
#let INSERT = "I"
#let REPLACE = "R"

#let wait(seconds: none) = {
	if seconds == none {
		panic("todo: wait until everything finished")
	}
	context {
		let fps = fps_state.get()
		let frames = fps * seconds
		for i in range(0, int(calc.ceil(frames))) {
			step()
		}
	}
}

#let text-diff-table(start, end) = {
	let table = ((0,) * (end.len() + 1),) * (start.len() + 1)
	for i in range(0, start.len() + 1) {
		table.at(i).at(0) = i
	}
	for j in range(0, end.len() + 1) {
		table.at(0).at(j) = j
	}

	for i in range(0, start.len()) {
		for j in range(0, end.len()) {
			table.at(i + 1).at(j + 1) = if start.at(i) == end.at(j) {
				table.at(i).at(j)
			} else {
				let a = table.at(i + 1).at(j)
				let b = table.at(i).at(j + 1)
				let c = table.at(i).at(j)
				calc.min(a, b, c) + 1
			}
		}
	}
	table
}

#let text-diff(table, start, end, i, j) = {
	if i == 0 and j == 0 {
		()
	} else if i == 0 {
		text-diff(table, start, end, i, j - 1)
		(INSERT,)
	} else if j == 0 {
		text-diff(table, start, end, i - 1, j)
		(DELETE,)
	} else if start.at(i - 1) == end.at(j - 1) {
		text-diff(table, start, end, i - 1, j - 1)
		(KEEP,)
	} else {
		let del = table.at(i).at(j - 1)
		let ins = table.at(i - 1).at(j)
		let repl = table.at(i).at(j)
		let best = calc.max(del, ins, repl)

		if repl == best {
			text-diff(table, start, end, i - 1, j - 1)
			(REPLACE,)
		} else if del > ins or (del == ins and i > j) {
			text-diff(table, start, end, i - 1, j)
			(DELETE,)
		} else {
			text-diff(table, start, end, i, j - 1)
			(INSERT,)
		}
	}
}

#let clusters-to-str(clusters) = {
	let str = ""
	for cluster in clusters {
		str += cluster
	}
	str
}

#let transform-text(start, end, delta: 0.1) = {
	let start = start.clusters()
	let end = end.clusters()
	let table = text-diff-table(start, end)
	let diff = text-diff(table, start, end, start.len(), end.len())
	let text = start
	let cursor = 0
	let variants = ((text, cursor),)
	for action in diff {
		if action == KEEP {
			cursor += 1
		} else if action == DELETE {
			text = text.slice(0, cursor) + text.slice(cursor + 1)
			variants.push((text, cursor))
		} else if action == INSERT {
			text = text.slice(0, cursor) + (end.at(cursor),) + text.slice(cursor)
			cursor += 1
			variants.push((text, cursor))
		} else if action == REPLACE{
			text = text.slice(0, cursor) + (end.at(cursor),) + text.slice(cursor + 1)
			cursor += 1
			variants.push((text, cursor))
		} else {
			panic()
		}
	}
	assert(text == end)
	for (variant, cursor) in variants {
		context {
			let fps = fps_state.get()
			spawn(timed(object(raw(block: true, clusters-to-str(variant), lang: "rust")), delta - 0.01))
			wait(seconds: delta)
		}
	}
}

#show: setup()

#scene("Test", background: rgb(20%, 30%, 40%), {
	spawn(timed(object([Hi]), 0.3))
	wait(seconds: 1.0)
	spawn(translate(x: 100pt, y: 100pt, object([Hi])))
	wait(seconds: 1.0)
})

#scene("Test 2", background: rgb(20%, 40%, 40%), {
	spawn(object([Hi]))
	wait(seconds: 1.0)
	spawn(translate(x: 100pt, y: 100pt, object([Hi])))
	wait(seconds: 1.0)
})

#scene("Test 3", background: rgb(100%, 40%, 40%), {
	spawn(object(`Hi`))
	wait(seconds: 1.0)
	spawn(translate(x: 100pt, y: 100pt, object([Hi])))
	wait(seconds: 1.0)
})

#let start-text = "
fn test() {
	other_function();
}"

#let end-text = "
fn main() {
	test_function(1.0);
}"

#scene("Transform Text", background: white, {
	transform-text(start-text, end-text, delta: 0.1)
})
