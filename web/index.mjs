import { h, Component, render } from 'https://unpkg.com/preact@latest?module';
import htm from 'https://unpkg.com/htm?module';

const html = htm.bind(h)

// polling version:
// const App = ({ cpus }) => html``
// setInterval(async () => {
//   const response = await fetch('/api/cpus')
//   if (!response.ok) {
//     console.error(
//       'failed to fetch CPU usage',
//       `status = ${response.status}`,
//       `text = ${await response.text()}`
//     )
//   }
//   const cpus = await response.json()

//   render(App({ cpus }), document.body)
// }, 200)

// websocket version:

const App = () =>
  html`
  <h1>CPU Usage</h1>

  <${CPUUsage}/>
  `

const CPUUsageBar = ({ cpu }) =>
  html`
  <div class="bar">
    <div class="bar-inner" style="width: ${cpu}%"></div>
    <label>${cpu.toFixed(2)}%</label>
  </div>`

class CPUUsage extends Component {
  constructor() {
    super()
    this.state = { cpus: [] }
  }

  attemptConnect () {
    const url = new URL('/realtime/cpus', window.location.href)
    url.protocol = url.protocol.replace('http', 'ws')
    return new WebSocket(url.href)
  }

  connect() {
    // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
    if(this.ws && this.ws.readyState !== 3) return

    this.ws = this.attemptConnect()
    this.ws.addEventListener('message', ({ data }) => {
      const cpus = JSON.parse(data)
      this.setState({ cpus })
    })
  }

  componentDidMount() {
    this.interval = setInterval(this.connect.bind(this), 1000)
  }

  componentWillUnmount() {
    if (this.ws) this.ws.close()
    if (this.interval) clearInterval(this.interval)
  }

  render(_props, state) {
    return html`
    <div>
      ${
        state.cpus.map(cpu =>
          html`<${CPUUsageBar} cpu="${cpu}" />`
        )
      }
    </div>
    `
  }
}

render(html`<${App} />`, document.body)
