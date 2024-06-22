#set document(title: "Example Project 2")
#let scenes = state("scenes", ())

#let fps_state = state("fps", 30)

// #let total = 10 * 24 - 1;
// #let total = 10 * 24;

// #for i in range(0, total) {

// }

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
		let scene = scene_state.get()
		rect(width: 100%, height: 100%, fill: scene.background, stroke: none, inset: 0pt, {
			for object in scene.objects {
				(object.render)()
			}
		})
	}

	scene_state.update(scene => {
		// update one frame
		scene
	})
}

#let setup(width: 1920pt, height: 1080pt, fps: 30) = {
	(doc) => {
		set page(width: width, height: height, margin: 0pt)
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
	render: () => place(content),
	alive: () => true,
	update: (delta) => { },
)

#let translate(x: 0pt, y: 0pt, object) = (
	render: () => place(dx: x, dy: y, (object.render)()),
	alive: object.alive,
	update: object.update,
)

#let timed() = { }

#let wait(seconds) = {
	context {
		let fps = fps_state.get()
		let frames = fps * seconds
		for i in range(0, int(frames)) {
			step()
		}
	}
}

#show: setup()

#scene("test", background: rgb(20%, 30%, 40%), {
	spawn(object([Hi]))
	wait(1.0)
	spawn(translate(x: 100pt, y: 100pt, object([Hi])))
	wait(1.0)
})

#scene("test-2", background: rgb(20%, 40%, 40%), {
	spawn(object([Hi]))
	wait(1.0)
	spawn(translate(x: 100pt, y: 100pt, object([Hi])))
	wait(1.0)
})

#scene("test-3", background: rgb(100%, 40%, 40%), {
	spawn(object([Hi]))
	wait(1.0)
	spawn(translate(x: 100pt, y: 100pt, object([Hi])))
	wait(1.0)
})
